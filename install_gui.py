#!/usr/bin/env python3
"""
Amni-Code GUI Installer
Graphical installer for Amni-Code AI assistant
"""

import tkinter as tk
from tkinter import ttk, scrolledtext, messagebox
import threading
import sys
import subprocess
import platform
import shutil
from pathlib import Path
import os

class GUInstaller:
    def __init__(self):
        self.root = tk.Tk()
        self.root.title("Amni-Code Agent Installer v0.3.0")
        self.root.geometry("700x600")
        self.root.resizable(True, True)
        
        # Modern window properties
        try:
            self.root.attributes('-alpha', 0.98)  # Glass-like transparency
            # Remove window borders for ultra-modern look (optional)
            # self.root.overrideredirect(True)
        except:
            pass
        
        # Center window on screen
        self.center_window()
        
        # Set theme
        style = ttk.Style()
        try:
            style.theme_use('vista')
        except:
            pass

        self.setup_ui()
        self.installer = AmniCodeInstaller()
        
        # Properly size window after all widgets are created
        self.root.update_idletasks()
        self.root.geometry("")  # Let tkinter calculate optimal size
        self.center_window()    # Re-center after sizing
        
        # Fade in animation
        self.fade_in()

    def fade_in(self):
        """Smooth fade-in animation"""
        try:
            alpha = 0.0
            while alpha < 0.98:
                alpha += 0.05
                self.root.attributes('-alpha', alpha)
                self.root.update()
                self.root.after(20)
        except:
            pass  # Fallback if transparency not supported

    def center_window(self):
        """Center the window on screen"""
        self.root.update_idletasks()
        width = self.root.winfo_width()
        height = self.root.winfo_height()
        x = (self.root.winfo_screenwidth() // 2) - (width // 2)
        y = (self.root.winfo_screenheight() // 2) - (height // 2)
        self.root.geometry(f'+{x}+{y}')

    def setup_ui(self):
        # Configure modern theme
        self.root.configure(bg='#1a1a1a')
        self.root.attributes('-alpha', 0.95)  # Slight transparency for glass effect
        
        # Set theme colors
        self.colors = {
            'bg_primary': '#1a1a1a',
            'bg_secondary': '#2d2d2d',
            'bg_accent': '#3d3d3d',
            'text_primary': '#ffffff',
            'text_secondary': '#cccccc',
            'accent': '#007acc',
            'accent_hover': '#0056b3',
            'success': '#28a745',
            'warning': '#ffc107',
            'error': '#dc3545',
            'progress_bg': '#404040',
            'progress_fg': '#007acc'
        }
        
        # Configure styles
        style = ttk.Style()
        style.configure('Modern.TFrame', background=self.colors['bg_primary'])
        style.configure('Card.TFrame', 
                       background=self.colors['bg_secondary'], 
                       relief='solid', 
                       borderwidth=1,
                       bordercolor=self.colors['bg_accent'])
        style.configure('Modern.TLabel', 
                       background=self.colors['bg_primary'], 
                       foreground=self.colors['text_primary'], 
                       font=('Segoe UI', 10))
        style.configure('Title.TLabel', 
                       background=self.colors['bg_primary'], 
                       foreground=self.colors['text_primary'], 
                       font=('Segoe UI', 18, 'bold'))
        style.configure('Status.TLabel', 
                       background=self.colors['bg_primary'], 
                       foreground=self.colors['text_secondary'], 
                       font=('Segoe UI', 9))
        style.configure('Modern.TButton', font=('Segoe UI', 10, 'bold'), padding=8)
        style.map('Modern.TButton',
                 background=[('active', self.colors['accent_hover']), ('pressed', self.colors['accent'])],
                 foreground=[('active', self.colors['text_primary'])])
        
        # Progress bar styling with modern look
        style.configure('Modern.Horizontal.TProgressbar',
                       background=self.colors['accent'],
                       troughcolor=self.colors['bg_secondary'],
                       borderwidth=0,
                       lightcolor=self.colors['accent'],
                       darkcolor=self.colors['accent'])
        
        # Main container with padding
        main_container = ttk.Frame(self.root, style='Modern.TFrame', padding="20")
        main_container.grid(row=0, column=0, sticky=(tk.W, tk.E, tk.N, tk.S))
        self.root.columnconfigure(0, weight=1)
        self.root.rowconfigure(0, weight=1)

        # Header section with glass effect
        header_frame = ttk.Frame(main_container, style='Card.TFrame', padding="20")
        header_frame.grid(row=0, column=0, sticky=(tk.W, tk.E), pady=(0, 20))
        header_frame.columnconfigure(0, weight=1)
        
        # App icon placeholder (text-based for now)
        icon_label = ttk.Label(header_frame, text="🚀", font=('Segoe UI', 48), style='Title.TLabel')
        icon_label.grid(row=0, column=0, pady=(0, 10))
        
        # Title
        title_label = ttk.Label(header_frame, text="Amni-Code Agent Installer",
                               style='Title.TLabel')
        title_label.grid(row=1, column=0, pady=(0, 5))
        
        # Subtitle
        subtitle_label = ttk.Label(header_frame, text="v0.3.0 • Intelligent AI Development Assistant",
                                  style='Status.TLabel')
        subtitle_label.grid(row=2, column=0)

        # Progress section
        progress_frame = ttk.Frame(main_container, style='Card.TFrame', padding="15")
        progress_frame.grid(row=1, column=0, sticky=(tk.W, tk.E), pady=(0, 20))
        progress_frame.columnconfigure(0, weight=1)

        # Progress label
        progress_title = ttk.Label(progress_frame, text="Installation Progress",
                                  style='Modern.TLabel', font=('Segoe UI', 12, 'bold'))
        progress_title.grid(row=0, column=0, sticky=tk.W, pady=(0, 10))

        # Progress bar with custom styling
        self.progress_var = tk.DoubleVar()
        self.progress_bar = ttk.Progressbar(progress_frame, variable=self.progress_var,
                                          maximum=100, mode='determinate',
                                          style='Modern.Horizontal.TProgressbar')
        self.progress_bar.grid(row=1, column=0, sticky=(tk.W, tk.E), pady=(0, 8))

        # Status label
        self.status_var = tk.StringVar(value="Ready to install Amni-Code")
        self.status_label = ttk.Label(progress_frame, textvariable=self.status_var,
                                     style='Status.TLabel')
        self.status_label.grid(row=2, column=0, sticky=tk.W)

        # Log section
        log_frame = ttk.Frame(main_container, style='Card.TFrame', padding="15")
        log_frame.grid(row=2, column=0, sticky=(tk.W, tk.E, tk.N, tk.S), pady=(0, 20))
        log_frame.columnconfigure(0, weight=1)
        log_frame.rowconfigure(0, weight=1)

        # Log title
        log_title = ttk.Label(log_frame, text="Installation Log",
                             style='Modern.TLabel', font=('Segoe UI', 12, 'bold'))
        log_title.grid(row=0, column=0, sticky=tk.W, pady=(0, 10))

        # Log text area with modern styling
        self.log_text = scrolledtext.ScrolledText(
            log_frame,
            height=12,
            wrap=tk.WORD,
            font=('Consolas', 9),
            bg=self.colors['bg_primary'],
            fg=self.colors['text_primary'],
            insertbackground=self.colors['text_primary'],
            selectbackground=self.colors['accent'],
            selectforeground=self.colors['text_primary'],
            relief='flat',
            borderwidth=0
        )
        self.log_text.grid(row=1, column=0, sticky=(tk.W, tk.E, tk.N, tk.S))

        # Configure scrollbar colors
        scrollbar = self.log_text.vbar
        scrollbar.configure(
            bg=self.colors['bg_secondary'],
            troughcolor=self.colors['bg_primary'],
            activebackground=self.colors['accent']
        )

        # Buttons section
        buttons_frame = ttk.Frame(main_container, style='Modern.TFrame')
        buttons_frame.grid(row=3, column=0, pady=(10, 0))

        # Left side - Install controls
        left_frame = ttk.Frame(buttons_frame, style='Modern.TFrame')
        left_frame.grid(row=0, column=0, sticky='w')

        # Install button with modern styling
        self.install_button = tk.Button(
            left_frame,
            text="🚀 Install Amni-Code",
            command=self.start_installation,
            font=('Segoe UI', 12, 'bold'),
            bg=self.colors['accent'],
            fg=self.colors['text_primary'],
            activebackground=self.colors['accent_hover'],
            activeforeground=self.colors['text_primary'],
            relief='flat',
            borderwidth=0,
            padx=25,
            pady=12,
            cursor='hand2',
            highlightthickness=0
        )
        self.install_button.grid(row=0, column=0, padx=(0, 15))

        # Cancel button
        self.cancel_button = tk.Button(
            left_frame,
            text="❌ Cancel",
            command=self.cancel_installation,
            font=('Segoe UI', 10),
            bg=self.colors['bg_secondary'],
            fg=self.colors['text_secondary'],
            activebackground=self.colors['error'],
            activeforeground=self.colors['text_primary'],
            relief='flat',
            borderwidth=0,
            padx=15,
            pady=8,
            cursor='hand2',
            state='disabled'
        )
        self.cancel_button.grid(row=0, column=1, padx=(0, 20))

        # Options section
        options_frame = ttk.Frame(left_frame, style='Modern.TFrame')
        options_frame.grid(row=1, column=0, columnspan=2, pady=(10, 0), sticky='w')

        # Desktop shortcut toggle
        self.shortcut_var = tk.BooleanVar(value=True)
        shortcut_check = tk.Checkbutton(
            options_frame,
            text="Create desktop shortcut",
            variable=self.shortcut_var,
            bg=self.colors['bg_primary'],
            fg=self.colors['text_primary'],
            selectcolor=self.colors['bg_secondary'],
            activebackground=self.colors['bg_primary'],
            activeforeground=self.colors['text_primary'],
            font=('Segoe UI', 9)
        )
        shortcut_check.grid(row=0, column=0, sticky='w')

        # API Key configuration section
        apikey_frame = ttk.Frame(main_container, style='Card.TFrame', padding="15")
        apikey_frame.grid(row=4, column=0, sticky=(tk.W, tk.E), pady=(0, 10))
        apikey_frame.columnconfigure(1, weight=1)
        apikey_title = ttk.Label(apikey_frame, text="API Key Setup (optional — configure now or later in Settings)",
                                style='Modern.TLabel', font=('Segoe UI', 11, 'bold'))
        apikey_title.grid(row=0, column=0, columnspan=3, sticky=tk.W, pady=(0, 10))
        providers = [
            ("xAI (Grok) — default", "XAI_API_KEY", "xai-..."),
            ("OpenAI", "OPENAI_API_KEY", "sk-..."),
            ("Anthropic", "ANTHROPIC_API_KEY", "sk-ant-..."),
        ]
        self.api_key_entries = {}
        for i, (label, env_var, placeholder) in enumerate(providers):
            lbl = tk.Label(apikey_frame, text=label, bg=self.colors['bg_secondary'],
                          fg=self.colors['text_secondary'], font=('Segoe UI', 9), anchor='w')
            lbl.grid(row=i+1, column=0, sticky='w', padx=(0, 10), pady=2)
            entry = tk.Entry(apikey_frame, bg=self.colors['bg_primary'], fg=self.colors['text_primary'],
                           insertbackground=self.colors['text_primary'], font=('Consolas', 9),
                           relief='flat', borderwidth=1, show='*')
            entry.insert(0, os.environ.get(env_var, ''))
            entry.grid(row=i+1, column=1, sticky='ew', padx=(0, 5), pady=2)
            show_btn = tk.Button(apikey_frame, text='👁', bg=self.colors['bg_secondary'],
                                fg=self.colors['text_secondary'], relief='flat', borderwidth=0,
                                font=('Segoe UI', 8), cursor='hand2',
                                command=lambda e=entry: e.config(show='' if e.cget('show') == '*' else '*'))
            show_btn.grid(row=i+1, column=2, padx=2, pady=2)
            self.api_key_entries[env_var] = entry
        hint = tk.Label(apikey_frame, text="Keys are saved to .env — never committed to git. You can also set them in the app's Settings panel.",
                       bg=self.colors['bg_secondary'], fg=self.colors['text_secondary'], font=('Segoe UI', 8))
        hint.grid(row=len(providers)+1, column=0, columnspan=3, sticky='w', pady=(6, 0))

        # Right side - Action buttons
        right_frame = ttk.Frame(buttons_frame, style='Modern.TFrame')
        right_frame.grid(row=0, column=1, sticky='e')

        # Launch button (initially hidden)
        self.launch_button = tk.Button(
            right_frame,
            text="🎯 Launch Amni-Code",
            command=self.launch_application,
            font=('Segoe UI', 11, 'bold'),
            bg=self.colors['success'],
            fg=self.colors['text_primary'],
            activebackground='#28a745',
            activeforeground=self.colors['text_primary'],
            relief='flat',
            borderwidth=0,
            padx=20,
            pady=10,
            cursor='hand2',
            highlightthickness=0
        )
        # Initially hidden
        self.launch_button.grid(row=0, column=0, padx=(0, 15))
        self.launch_button.grid_remove()

        # Exit button
        exit_button = tk.Button(
            right_frame,
            text="Exit",
            command=self.root.quit,
            font=('Segoe UI', 10),
            bg=self.colors['bg_primary'],
            fg=self.colors['text_secondary'],
            activebackground=self.colors['bg_secondary'],
            activeforeground=self.colors['text_primary'],
            relief='flat',
            borderwidth=0,
            padx=15,
            pady=8,
            cursor='hand2'
        )
        exit_button.grid(row=0, column=1)

        # Configure grid weights
        main_container.columnconfigure(0, weight=1)
        main_container.rowconfigure(2, weight=1)
        
        # Configure buttons frame
        buttons_frame.columnconfigure(0, weight=1)  # Left frame expands
        buttons_frame.columnconfigure(1, weight=0)  # Right frame stays fixed

        # Add hover effects
        self.add_hover_effects()

    def add_hover_effects(self):
        """Add hover effects to buttons"""
        def on_enter_install(e):
            if self.install_button.cget('state') != 'disabled':
                self.install_button.config(bg=self.colors['accent_hover'])
        
        def on_leave_install(e):
            if self.install_button.cget('state') != 'disabled':
                self.install_button.config(bg=self.colors['accent'])
        
        def on_enter_cancel(e):
            if self.cancel_button.cget('state') != 'disabled':
                self.cancel_button.config(bg=self.colors['error'])
        
        def on_leave_cancel(e):
            if self.cancel_button.cget('state') != 'disabled':
                self.cancel_button.config(bg=self.colors['bg_secondary'])

        self.install_button.bind("<Enter>", on_enter_install)
        self.install_button.bind("<Leave>", on_leave_install)
        self.cancel_button.bind("<Enter>", on_enter_cancel)
        self.cancel_button.bind("<Leave>", on_leave_cancel)

    def show_launch_button(self):
        """Show the launch button after successful installation"""
        self.launch_button.grid()
        # Add hover effect for launch button
        def on_enter_launch(e):
            self.launch_button.config(bg='#218838')
        def on_leave_launch(e):
            self.launch_button.config(bg=self.colors['success'])
        
        self.launch_button.bind("<Enter>", on_enter_launch)
        self.launch_button.bind("<Leave>", on_leave_launch)

    def launch_application(self):
        """Launch the installed Amni-Code application"""
        exe_path = Path(__file__).parent / "target" / "release" / "amni-code.exe"
        
        # Check if executable exists
        if not exe_path.exists():
            error_msg = f"Application executable not found at: {exe_path}"
            self.log(error_msg)
            messagebox.showerror("Launch Failed", error_msg)
            return
        
        # Check if it's actually executable
        if not exe_path.is_file():
            error_msg = f"Path exists but is not a file: {exe_path}"
            self.log(error_msg)
            messagebox.showerror("Launch Failed", error_msg)
            return
        
        try:
            self.log(f"Launching Amni-Code from: {exe_path}")
            
            # Launch the application
            import subprocess
            process = subprocess.Popen(
                [str(exe_path)], 
                cwd=Path(__file__).parent,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                creationflags=subprocess.CREATE_NO_WINDOW if hasattr(subprocess, 'CREATE_NO_WINDOW') else 0
            )
            
            self.log("Amni-Code launched successfully!")
            messagebox.showinfo("Success", "Amni-Code is launching!\n\nCheck for the application window to open.")
            
        except FileNotFoundError:
            error_msg = f"Executable not found: {exe_path}"
            self.log(f"Launch failed: {error_msg}")
            messagebox.showerror("Launch Failed", error_msg)
            
        except PermissionError:
            error_msg = f"Permission denied executing: {exe_path}"
            self.log(f"Launch failed: {error_msg}")
            messagebox.showerror("Launch Failed", f"Permission denied.\n\nTry running as administrator or check file permissions for:\n{exe_path}")
            
        except Exception as e:
            error_msg = f"Failed to launch application: {str(e)}"
            self.log(error_msg)
            messagebox.showerror("Launch Failed", f"Could not launch Amni-Code:\n{str(e)}\n\nExecutable: {exe_path}")

    def log(self, message):
        self.log_text.insert(tk.END, message + "\n")
        self.log_text.see(tk.END)
        self.root.update_idletasks()

    def update_progress(self, value, status):
        self.progress_var.set(value)
        self.status_var.set(status)
        self.root.update_idletasks()

    def start_installation(self):
        self.install_button.config(state="disabled")
        self.cancel_button.config(state="normal")
        self.installation_cancelled = False

        # Start installation in a separate thread
        install_thread = threading.Thread(target=self.run_installation)
        install_thread.daemon = True
        install_thread.start()

    def cancel_installation(self):
        self.installation_cancelled = True
        self.cancel_button.config(state="disabled")
        self.log("Installation cancelled by user.")

    def run_installation(self):
        try:
            self.log("=" * 50)
            self.log("   Amni-Code Agent Installer v0.3.0")
            self.log("=" * 50)
            self.log("")

            total_steps = 8
            current_step = 0

            # Step 1: Check Rust
            if self.installation_cancelled:
                return
            current_step += 1
            self.update_progress((current_step / total_steps) * 100, f"Step {current_step}/8: Checking Rust")
            if not self.installer.check_rust():
                self.show_error("Rust installation failed. Please install Rust and try again.")
                return

            # Step 2: Check Python
            if self.installation_cancelled:
                return
            current_step += 1
            self.update_progress((current_step / total_steps) * 100, f"Step {current_step}/8: Checking Python")
            if not self.installer.check_python():
                self.show_error("Python 3.13 not found. Please install Python 3.13 and try again.")
                return

            # Step 3: Detect hardware
            if self.installation_cancelled:
                return
            current_step += 1
            self.update_progress((current_step / total_steps) * 100, f"Step {current_step}/8: Detecting hardware")
            self.installer.detect_hardware()

            # Step 3.5: Save API keys to .env
            self.save_api_keys()

            # Step 4: Install Python deps
            if self.installation_cancelled:
                return
            current_step += 1
            self.update_progress((current_step / total_steps) * 100, f"Step {current_step}/8: Installing dependencies")
            if not self.installer.install_python_deps():
                self.show_error("Failed to install Python dependencies.")
                return

            # Step 5: Build Rust app
            if self.installation_cancelled:
                return
            current_step += 1
            self.update_progress((current_step / total_steps) * 100, f"Step {current_step}/8: Building application")
            if not self.installer.build_rust_app():
                self.show_error("Failed to build Rust application.")
                return

            # Step 6: Setup models dir
            if self.installation_cancelled:
                return
            current_step += 1
            self.update_progress((current_step / total_steps) * 100, f"Step {current_step}/8: Setting up directories")
            self.installer.setup_models_dir()

            # Step 7: Download models (optional)
            if self.installation_cancelled:
                return
            current_step += 1
            self.update_progress((current_step / total_steps) * 100, f"Step {current_step}/8: Downloading models")
            self.installer.download_models()

            # Step 8: Create shortcut (optional)
            if self.installation_cancelled:
                return
            current_step += 1
            self.update_progress((current_step / total_steps) * 100, f"Step {current_step}/8: Finalizing")
            
            if self.shortcut_var.get():
                self.installer.create_shortcut()
            else:
                self.log("Desktop shortcut creation skipped (per user preference).")

            # Success
            self.update_progress(100, "Installation completed successfully!")
            self.log("\n" + "=" * 50)
            self.log("Installation completed successfully!")
            self.log("You can now run Amni-Code from: target/release/amni-code.exe")
            if self.shortcut_var.get():
                self.log("A desktop shortcut has been created for easy access.")
            self.log("=" * 50)

            # Show launch button
            self.show_launch_button()

            messagebox.showinfo("Installation Complete",
                              "Amni-Code has been installed successfully!\n\n"
                              "Click 'Launch Amni-Code' to start using it right away.")

        except Exception as e:
            self.show_error(f"Installation failed: {str(e)}")

        finally:
            self.install_button.config(state="normal")
            self.cancel_button.config(state="disabled")

    def show_error(self, message):
        self.log(f"ERROR: {message}")
        messagebox.showerror("Installation Failed", message)

    def save_api_keys(self):
        env_path = Path(__file__).parent / ".env"
        lines = []
        for env_var, entry in self.api_key_entries.items():
            val = entry.get().strip()
            if val:
                lines.append(f"{env_var}={val}")
                os.environ[env_var] = val
        if lines:
            env_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
            self.log(f"Saved {len(lines)} API key(s) to .env")
        else:
            self.log("No API keys configured — you can add them later in Settings.")

class AmniCodeInstaller:
    def __init__(self):
        self.project_root = Path(__file__).parent
        self.has_nvidia = False
        self.has_amd = False
        self.has_cuda = False
        self.has_hip = False

    def run_command(self, cmd, description="", check=True, quiet=False):
        """Run a command and return success status"""
        if not quiet:
            print(f"[INFO] {description}")
        if isinstance(cmd, str):
            cmd = cmd.split()

        # Use Python 3.13 specifically for pip commands
        if len(cmd) >= 2 and cmd[0] == "python" and cmd[1] == "-m" and cmd[2] == "pip":
            cmd = ["py", "-3.13", "-m", "pip"] + cmd[3:]
        elif len(cmd) >= 1 and cmd[0] == "python":
            cmd = ["py", "-3.13"] + cmd[1:]

        try:
            result = subprocess.run(cmd, capture_output=True, text=True, cwd=self.project_root)
            success = result.returncode == 0
            if check and not success:
                print(f"[ERROR] Command failed: {' '.join(cmd)}")
                print(f"[ERROR] {result.stderr}")
                return False
            return success
        except Exception as e:
            if check:
                print(f"[ERROR] Failed to run command: {e}")
            return False

    def check_rust(self):
        """Check for Rust and install if needed"""
        print("\n[1/8] Checking for Rust toolchain...")
        if self.run_command("rustc --version", check=False):
            print("Rust is already installed.")
            return True

        print("Rust not found. Installing...")
        rustup_cmd = 'curl --proto \'=https\' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y'
        if platform.system() == "Windows":
            rustup_cmd = "curl -sSf https://sh.rustup.rs | sh -s -- -y"
        return self.run_command(rustup_cmd, "Installing Rust")

    def check_python(self):
        """Check for Python"""
        print("\n[2/8] Checking for Python...")

        # Check if Python 3.13 is available
        try:
            result = subprocess.run(["py", "-3.13", "--version"], capture_output=True, text=True)
            if result.returncode == 0:
                version = result.stdout.strip()
                print(f"Python 3.13 detected: {version}")
                print("Using Python 3.13 for installation.")
                return True
            else:
                print("Python 3.13 not found.")
                print("Please install Python 3.13 from https://python.org")
                return False
        except Exception as e:
            print(f"Could not check for Python 3.13: {e}")
            return False

    def detect_hardware(self):
        """Detect hardware acceleration"""
        print("\n[3/8] Detecting hardware acceleration...")

        # Check NVIDIA
        try:
            result = subprocess.run(["nvidia-smi"], capture_output=True, text=True)
            if result.returncode == 0:
                self.has_nvidia = True
                self.has_cuda = True
                print("NVIDIA GPU with CUDA detected.")
        except:
            pass

        # Check AMD
        rocm_paths = [Path("C:/Program Files/AMD/ROCm"), Path("/opt/rocm")]
        for path in rocm_paths:
            if path.exists():
                self.has_amd = True
                self.has_hip = True
                print(f"AMD GPU with HIP/ROCm detected ({'WSL' if '/opt' in str(path) else 'Windows'}).")
                break
        else:
            # Try WMIC detection
            try:
                result = subprocess.run(["wmic", "path", "win32_VideoController", "get", "name"],
                                      capture_output=True, text=True)
                if "AMD" in result.stdout.upper() or "RADEON" in result.stdout.upper():
                    self.has_amd = True
                    print("AMD GPU detected but HIP/ROCm not installed.")
            except:
                pass

        if not self.has_nvidia and not self.has_amd:
            print("No GPU detected. Running in CPU mode.")

        return True

    def install_python_deps(self):
        """Install Python dependencies"""
        print("\n[4/8] Installing Python dependencies...")

        # First upgrade pip
        if not self.run_command("py -3.13 -m pip install --upgrade pip", check=False):
            print("Warning: Could not upgrade pip, continuing...")

        # Install PyTorch (separate because it has special index)
        print("Installing PyTorch...")
        pytorch_cmd = "py -3.13 -m pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu121"
        pytorch_success = self.run_command(pytorch_cmd, check=False)

        # Install core ML packages
        print("Installing core ML packages...")
        ml_cmd = "py -3.13 -m pip install huggingface_hub transformers accelerate safetensors"
        ml_success = self.run_command(ml_cmd, check=False)

        # Install huggingface-cli for model downloads
        print("Installing HuggingFace CLI...")
        hf_cmd = "py -3.13 -m pip install huggingface_hub[cli]"
        hf_success = self.run_command(hf_cmd, check=False)

        if not (pytorch_success and ml_success):
            print("Some packages failed to install.")
            return False

        print("Python dependencies installed successfully.")
        return True

    def build_rust_app(self):
        """Build the Rust application"""
        print("\n[5/8] Building Rust application...")
        if self.run_command("cargo build --release", "Building Amni-Code"):
            print("Rust application built successfully.")
            return True
        else:
            print("Build failed! Please check Rust installation.")
            return False

    def setup_models_dir(self):
        """Create models directory"""
        print("\n[6/8] Setting up models directory...")
        models_dir = self.project_root / "models"
        models_dir.mkdir(exist_ok=True)
        print("Models directory created.")
        return True

    def download_models(self):
        """Download AI models"""
        print("\n[7/8] Downloading AI models...")
        print("This may take several minutes depending on your internet speed...")

        models = [
            ("Jackrong/Qwen3.5-9B-Neo", "models/Qwen3.5-9B-Neo"),
            ("Jackrong/MLX-Qwen3.5-4B-Claude-4.6-Opus-Reasoning-Distilled-8bit", "models/MLX-Qwen3.5-4B")
        ]

        for model_repo, local_dir in models:
            print(f"\nDownloading {model_repo}...")
            cmd = f"huggingface-cli download {model_repo} --local-dir {local_dir} --local-dir-use-symlinks False"
            if not self.run_command(cmd, f"Downloading {model_repo}", check=False):
                print(f"Failed to download {model_repo}. You can retry later or download manually.")

        print("\nModel downloads completed.")
        return True

    def create_shortcut(self):
        """Create desktop shortcut"""
        print("\n[8/8] Creating desktop shortcut...")
        try:
            import winshell
            from win32com.client import Dispatch

            exe_path = self.project_root / "target" / "release" / "amni-code.exe"
            desktop = winshell.desktop()
            shortcut_path = os.path.join(desktop, "Amni-Code.lnk")

            shell = Dispatch('WScript.Shell')
            shortcut = shell.CreateShortCut(shortcut_path)
            shortcut.Targetpath = str(exe_path)
            shortcut.WorkingDirectory = str(self.project_root)
            shortcut.save()

            print("Desktop shortcut created.")
            return True
        except ImportError:
            print("winshell/win32com not available, trying alternative method...")
            return self.create_shortcut_fallback()
        except Exception as e:
            print(f"Failed to create shortcut: {e}")
            return self.create_shortcut_fallback()

    def create_shortcut_fallback(self):
        """Create desktop shortcut using PowerShell as fallback"""
        try:
            exe_path = self.project_root / "target" / "release" / "amni-code.exe"
            desktop_path = os.path.join(os.path.expanduser("~"), "Desktop")
            shortcut_path = os.path.join(desktop_path, "Amni-Code.lnk")
            
            # Use PowerShell to create shortcut
            ps_command = f'''
            $WshShell = New-Object -comObject WScript.Shell
            $Shortcut = $WshShell.CreateShortcut("{shortcut_path}")
            $Shortcut.TargetPath = "{exe_path}"
            $Shortcut.WorkingDirectory = "{self.project_root}"
            $Shortcut.Save()
            '''
            
            result = subprocess.run(["powershell", "-Command", ps_command], 
                                  capture_output=True, text=True)
            
            if result.returncode == 0:
                print("Desktop shortcut created (using PowerShell).")
                return True
            else:
                print(f"PowerShell shortcut creation failed: {result.stderr}")
                print(f"To create a shortcut manually, create a shortcut to: {exe_path}")
                return False
                
        except Exception as e:
            print(f"Fallback shortcut creation failed: {e}")
            exe_path = self.project_root / "target" / "release" / "amni-code.exe"
            print(f"To create a shortcut manually, create a shortcut to: {exe_path}")
            return False

def main():
    app = GUInstaller()
    app.root.mainloop()

if __name__ == "__main__":
    main()