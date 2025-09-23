using System;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Shapes;

namespace CommandSidekick.SpinnerPlugin
{
    /// <summary>
    /// Interaction logic for MainWindow.xaml
    /// </summary>
    public partial class MainWindow : Window
    {
        private readonly CancellationTokenSource _cancellationTokenSource = new();
        private Storyboard? _spinAnimation;

        public MainWindow()
        {
            InitializeComponent();
            SetupWindow();
            CreateSpinner();
            StartStdinMonitoring();
        }

        private void SetupWindow()
        {
            // Get environment variables from the Core Service
            string command = Environment.GetEnvironmentVariable("SIDEKICK_COMMAND") ?? "Unknown Command";
            string pid = Environment.GetEnvironmentVariable("SIDEKICK_PID") ?? "Unknown PID";

            // Setup window properties
            this.Title = "Command Sidekick - In Progress";
            this.Width = 300;
            this.Height = 200;
            this.WindowStartupLocation = WindowStartupLocation.CenterScreen;
            this.Topmost = true;
            this.ResizeMode = ResizeMode.NoResize;
            this.ShowInTaskbar = false;

            // Set background to semi-transparent dark
            this.Background = new SolidColorBrush(Color.FromArgb(200, 30, 30, 30));
            this.AllowsTransparency = true;
            this.WindowStyle = WindowStyle.None;

            // Round corners
            this.Background = new SolidColorBrush(Color.FromArgb(240, 40, 40, 40));
            
            // Create main layout
            var mainGrid = new Grid();
            this.Content = mainGrid;

            // Title
            var titleLabel = new Label
            {
                Content = "Command Running...",
                Foreground = Brushes.White,
                FontSize = 16,
                FontWeight = FontWeights.Bold,
                HorizontalAlignment = HorizontalAlignment.Center,
                VerticalAlignment = VerticalAlignment.Top,
                Margin = new Thickness(0, 20, 0, 0)
            };
            mainGrid.Children.Add(titleLabel);

            // Command info
            var commandLabel = new Label
            {
                Content = $"PID: {pid}",
                Foreground = Brushes.LightGray,
                FontSize = 10,
                HorizontalAlignment = HorizontalAlignment.Center,
                VerticalAlignment = VerticalAlignment.Top,
                Margin = new Thickness(0, 45, 0, 0)
            };
            mainGrid.Children.Add(commandLabel);

            // Spinner container
            var spinnerContainer = new Canvas
            {
                Width = 60,
                Height = 60,
                HorizontalAlignment = HorizontalAlignment.Center,
                VerticalAlignment = VerticalAlignment.Center,
                Margin = new Thickness(0, 20, 0, 0)
            };
            mainGrid.Children.Add(spinnerContainer);

            // Create the spinner
            CreateSpinnerInContainer(spinnerContainer);
        }

        private void CreateSpinner()
        {
            // This method is kept for backwards compatibility
            // The actual spinner is created in SetupWindow
        }

        private void CreateSpinnerInContainer(Canvas container)
        {
            const int dotCount = 8;
            const double radius = 20;
            const double dotSize = 6;

            for (int i = 0; i < dotCount; i++)
            {
                var angle = i * 2 * Math.PI / dotCount;
                var x = radius * Math.Cos(angle) + container.Width / 2 - dotSize / 2;
                var y = radius * Math.Sin(angle) + container.Height / 2 - dotSize / 2;

                var dot = new Ellipse
                {
                    Width = dotSize,
                    Height = dotSize,
                    Fill = new SolidColorBrush(Color.FromRgb(100, 149, 237)) // Cornflower blue
                };

                Canvas.SetLeft(dot, x);
                Canvas.SetTop(dot, y);
                container.Children.Add(dot);

                // Create fade animation for each dot
                var fadeAnimation = new DoubleAnimation
                {
                    From = 0.2,
                    To = 1.0,
                    Duration = TimeSpan.FromMilliseconds(800),
                    BeginTime = TimeSpan.FromMilliseconds(i * 100),
                    RepeatBehavior = RepeatBehavior.Forever,
                    AutoReverse = true
                };

                dot.BeginAnimation(UIElement.OpacityProperty, fadeAnimation);
            }
        }

        private async void StartStdinMonitoring()
        {
            await Task.Run(async () =>
            {
                try
                {
                    using var reader = new StreamReader(Console.OpenStandardInput());
                    
                    // This will block until stdin is closed by the Core Service
                    await reader.ReadToEndAsync();
                    
                    // Stdin was closed, meaning the command finished
                    await Dispatcher.InvokeAsync(() =>
                    {
                        ShowCompletionMessage();
                    });
                }
                catch (Exception ex)
                {
                    // Log error (in a real app, you'd use proper logging)
                    Console.WriteLine($"Error monitoring stdin: {ex.Message}");
                    
                    await Dispatcher.InvokeAsync(() =>
                    {
                        Close();
                    });
                }
            }, _cancellationTokenSource.Token);
        }

        private async void ShowCompletionMessage()
        {
            // Change the UI to show completion
            if (this.Content is Grid mainGrid)
            {
                mainGrid.Children.Clear();

                var completionLabel = new Label
                {
                    Content = "✓ Command Completed!",
                    Foreground = Brushes.LightGreen,
                    FontSize = 18,
                    FontWeight = FontWeights.Bold,
                    HorizontalAlignment = HorizontalAlignment.Center,
                    VerticalAlignment = VerticalAlignment.Center
                };
                mainGrid.Children.Add(completionLabel);
            }

            // Wait a moment before closing
            await Task.Delay(2000);
            Close();
        }

        protected override void OnClosed(EventArgs e)
        {
            _cancellationTokenSource.Cancel();
            _spinAnimation?.Stop();
            base.OnClosed(e);
        }
    }
}