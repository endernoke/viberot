using System;
using System.IO;
using System.IO.Pipes;
using System.Management;
using System.Threading;
using System.Threading.Tasks;
using Newtonsoft.Json;

namespace CommandSidekick.WindowsProbe
{
    public class ProcessEvent
    {
        public uint pid { get; set; }
        public string command { get; set; } = string.Empty;
        public ulong timestamp { get; set; }
    }

    public class WindowsProbe
    {
        private const string PipeName = "command-sidekick-events";
        private ManagementEventWatcher? _processWatcher;
        private NamedPipeClientStream? _pipeClient;
        private StreamWriter? _pipeWriter;
        private readonly CancellationTokenSource _cancellationTokenSource = new();

        public async Task StartAsync()
        {
            Console.WriteLine("Command-Sidekick Windows Probe starting...");

            try
            {
                await ConnectToPipeAsync();
                StartProcessMonitoring();

                // Keep the application running
                await Task.Delay(Timeout.Infinite, _cancellationTokenSource.Token);
            }
            catch (OperationCanceledException)
            {
                Console.WriteLine("Probe stopped.");
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error: {ex.Message}");
                throw;
            }
            finally
            {
                Cleanup();
            }
        }

        private async Task ConnectToPipeAsync()
        {
            const int maxRetries = 10;
            const int retryDelayMs = 1000;

            for (int attempt = 1; attempt <= maxRetries; attempt++)
            {
                try
                {
                    Console.WriteLine($"Attempting to connect to named pipe (attempt {attempt}/{maxRetries})...");
                    
                    _pipeClient = new NamedPipeClientStream(".", PipeName, PipeDirection.Out);
                    await _pipeClient.ConnectAsync(5000, _cancellationTokenSource.Token);
                    
                    _pipeWriter = new StreamWriter(_pipeClient) { AutoFlush = true };
                    Console.WriteLine("Connected to Core Service via named pipe.");
                    return;
                }
                catch (TimeoutException)
                {
                    Console.WriteLine($"Connection attempt {attempt} timed out.");
                    _pipeClient?.Dispose();
                    _pipeClient = null;
                }
                catch (Exception ex)
                {
                    Console.WriteLine($"Connection attempt {attempt} failed: {ex.Message}");
                    _pipeClient?.Dispose();
                    _pipeClient = null;
                }

                if (attempt < maxRetries)
                {
                    await Task.Delay(retryDelayMs, _cancellationTokenSource.Token);
                }
            }

            throw new InvalidOperationException("Failed to connect to Core Service after multiple attempts. Ensure the Core Service is running.");
        }

        private void StartProcessMonitoring()
        {
            Console.WriteLine("Starting WMI process monitoring...");

            try
            {
                // WMI query to monitor process creation and termination
                const string query = "SELECT * FROM __InstanceOperationEvent WITHIN 1 WHERE TargetInstance ISA 'Win32_Process'";
                
                _processWatcher = new ManagementEventWatcher(query);
                _processWatcher.EventArrived += OnProcessEvent;
                _processWatcher.Start();

                Console.WriteLine("WMI process monitoring started successfully.");
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Failed to start process monitoring: {ex.Message}");
                throw;
            }
        }

        private async void OnProcessEvent(object sender, EventArrivedEventArgs e)
        {
            try
            {
                string eventType = e.NewEvent.ClassPath.ClassName;
                var targetInstance = e.NewEvent["TargetInstance"] as ManagementBaseObject;

                if (targetInstance == null) return;

                // Only handle process creation events for now
                if (eventType == "__InstanceCreationEvent")
                {
                    uint processId = Convert.ToUInt32(targetInstance["ProcessId"]);
                    string processName = targetInstance["Name"]?.ToString() ?? "";
                    string commandLine = targetInstance["CommandLine"]?.ToString() ?? processName;

                    // Create event object
                    var processEvent = new ProcessEvent
                    {
                        pid = processId,
                        command = commandLine,
                        timestamp = (ulong)DateTimeOffset.UtcNow.ToUnixTimeSeconds()
                    };

                    // Send to Core Service
                    await SendEventAsync(processEvent);
                }
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error processing WMI event: {ex.Message}");
            }
        }

        private async Task SendEventAsync(ProcessEvent processEvent)
        {
            try
            {
                if (_pipeWriter == null)
                {
                    Console.WriteLine("Pipe writer is not available. Attempting to reconnect...");
                    await ConnectToPipeAsync();
                }

                if (_pipeWriter != null)
                {
                    string json = JsonConvert.SerializeObject(processEvent);
                    await _pipeWriter.WriteLineAsync(json);
                    await _pipeWriter.FlushAsync(); // Ensure data is sent immediately
                    
                    // Only log specific commands we're interested in to avoid spam
                    if (processEvent.command.Contains("npm") || 
                        processEvent.command.Contains("docker") || 
                        processEvent.command.Contains("git"))
                    {
                        Console.WriteLine($"Sent event: PID={processEvent.pid}, Command={processEvent.command}");
                    }
                }
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Failed to send event: {ex.Message}");
                
                // Try to reconnect on next event
                _pipeWriter?.Dispose();
                _pipeWriter = null;
                _pipeClient?.Dispose();
                _pipeClient = null;
            }
        }

        private void Cleanup()
        {
            Console.WriteLine("Cleaning up resources...");

            try
            {
                _processWatcher?.Stop();
                _processWatcher?.Dispose();
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error stopping process watcher: {ex.Message}");
            }

            try
            {
                _pipeWriter?.Dispose();
                _pipeClient?.Dispose();
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error closing pipe: {ex.Message}");
            }
        }

        public void Stop()
        {
            _cancellationTokenSource.Cancel();
        }
    }

    class Program
    {
        static async Task Main(string[] args)
        {
            Console.WriteLine("Command-Sidekick Windows Probe v0.1.0");
            Console.WriteLine("Monitoring process creation events...");
            Console.WriteLine("Press Ctrl+C to exit.");

            var probe = new WindowsProbe();

            // Handle Ctrl+C gracefully
            Console.CancelKeyPress += (_, e) =>
            {
                e.Cancel = true;
                probe.Stop();
            };

            try
            {
                await probe.StartAsync();
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Probe failed: {ex.Message}");
                Environment.Exit(1);
            }
        }
    }
}