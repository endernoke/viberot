using System;
using System.IO;
using System.Windows;

namespace CommandSidekick.SpinnerPlugin
{
    /// <summary>
    /// Interaction logic for App.xaml
    /// </summary>
    public partial class App : Application
    {
        protected override void OnStartup(StartupEventArgs e)
        {
            base.OnStartup(e);

            // Log startup for debugging
            try
            {
                var logMessage = $"[{DateTime.Now:yyyy-MM-dd HH:mm:ss}] Spinner Plugin started. " +
                               $"PID: {Environment.GetEnvironmentVariable("SIDEKICK_PID")}, " +
                               $"Command: {Environment.GetEnvironmentVariable("SIDEKICK_COMMAND")}";
                
                // Create a simple log in temp folder for debugging
                var tempPath = Path.GetTempPath();
                var logFile = Path.Combine(tempPath, "command-sidekick-spinner.log");
                File.AppendAllText(logFile, logMessage + Environment.NewLine);
            }
            catch (Exception ex)
            {
                // Ignore logging errors
                Console.WriteLine($"Logging error: {ex.Message}");
            }

            // Create and show the main window
            var mainWindow = new MainWindow();
            mainWindow.Show();
        }
    }
}