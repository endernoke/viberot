# Project Plan: VibeRot

This document outlines the project plan and architecture for "VibeRot," a configurable utility to run actions based on command execution.

## 1. Vision

To create a fun, powerful, and cross-platform utility that allows users to trigger custom actions (e.g., show a UI, play music, run a script) when specific command-line processes (like `npm install` or `git push`) are executed.

## 2. Core Architecture

The proposed architecture is a modular, three-layer system designed for performance, flexibility, and cross-platform compatibility.

- **[Core Service](./core_service.md)**: A high-performance background service written in Rust. It acts as the central engine, managing configuration, and orchestrating actions.
- **[Platform Probes](./platform_probes.md)**: Lightweight, OS-specific modules that efficiently detect process creation events using native, high-performance APIs.
- **[Action Plugins](./action_plugins.md)**: User-defined scripts or applications that are triggered by the Core Service.

## 3. Development Roadmap

The project will be developed in phases, starting with a minimal viable product (MVP) and progressively adding features.

See the detailed [Development Milestones](./milestones.md) for more information.

## 4. Key Technology Choices

- **Core Language**: **Rust**. For its performance, safety, and excellent cross-platform capabilities.
- **Process Monitoring**:
    - **Linux**: eBPF
    - **Windows**: Event Tracing for Windows (ETW)
    - **macOS**: DTrace
- **User Scripting**: **Lua**. Embedded within the Core Service for simple, fast, and dependency-free user scripts. External executables will also be supported for more complex actions.
- **Configuration**: **TOML**. For its human-readable and straightforward syntax.
