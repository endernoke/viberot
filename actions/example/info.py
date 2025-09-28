#!/usr/bin/env python3
"""
Test script to demonstrate viberot action plugin functionality.
This script will be triggered when a monitored command runs.
"""

import os
import sys
import time
import tkinter as tk
from tkinter import ttk
import threading

def create_gui():
    root = tk.Tk()
    root.title("VibeRot Action Plugin")
    root.geometry("500x300")
    
    # Main frame
    main_frame = ttk.Frame(root, padding="10")
    main_frame.grid(row=0, column=0, sticky=(tk.W, tk.E, tk.N, tk.S))
    
    # Title
    title_label = ttk.Label(main_frame, text="=== VibeRot Action Plugin Test ===", 
                           font=("Arial", 12, "bold"))
    title_label.grid(row=0, column=0, columnspan=2, pady=(0, 10))
    
    # Information labels
    info_data = [
        ("PID of monitored process:", os.getenv('VIBEROT_PID', 'Not set')),
        ("Command that was run:", os.getenv('VIBEROT_COMMAND', 'Not set')),
        ("Timestamp:", os.getenv('VIBEROT_TIMESTAMP', 'Not set')),
        ("VibeRot Home:", os.getenv('VIBEROT_HOME', 'Not set')),
        ("Current working directory:", os.getcwd()),
        ("Script location:", os.path.abspath(__file__))
    ]
    
    for i, (label_text, value_text) in enumerate(info_data, start=1):
        ttk.Label(main_frame, text=label_text, font=("Arial", 9, "bold")).grid(
            row=i, column=0, sticky=tk.W, pady=2)
        ttk.Label(main_frame, text=value_text, wraplength=300).grid(
            row=i, column=1, sticky=tk.W, padx=(10, 0), pady=2)
    
    # Status label
    status_label = ttk.Label(main_frame, text="Waiting for monitored command to finish...", 
                            font=("Arial", 9, "italic"))
    status_label.grid(row=len(info_data)+2, column=0, columnspan=2, pady=(10, 0))
    
    # Configure grid weights
    root.columnconfigure(0, weight=1)
    root.rowconfigure(0, weight=1)
    main_frame.columnconfigure(1, weight=1)
    
    return root, status_label

def monitor_stdin(status_label, root):
    """Monitor stdin in a separate thread"""
    try:
        for line in sys.stdin:
            pass
    except KeyboardInterrupt:
        pass
    
    # Update GUI on main thread
    root.after(0, lambda: status_label.config(text="Monitored command has finished. Closing in 3 seconds..."))
    root.after(2000, root.quit)

def main():
    root, status_label = create_gui()
    
    # Start stdin monitoring in separate thread
    monitor_thread = threading.Thread(target=monitor_stdin, args=(status_label, root), daemon=True)
    monitor_thread.start()
    
    # Start GUI
    root.mainloop()

if __name__ == "__main__":
    main()