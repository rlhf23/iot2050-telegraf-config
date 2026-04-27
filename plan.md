# TUI Device Lifecycle Integration Plan

## Goal
Add device lifecycle management (provision, setup, update, start, stop, status, backup, restore) to the TUI, and fix the event loop to support long-running operations.

## Phase 1: Fix Event Loop (Prerequisites)
- [x] 1.1 Switch from blocking `event::read()` to `event::poll(timeout)` in `run_app` loop
- [x] 1.2 Change `process_worker_responses` to drain all responses per cycle (not just one)
- [x] 1.3 Compile and verify

## Phase 2: Add DeploymentConfig to App State
- [x] 2.1 Add `device_name`, `key_file`, `git_branch` fields to App
- [x] 2.2 Add `minimal` and `local_transfer` flags to `App` for device operations
- [x] 2.3 Create a `DeploymentConfig` builder method on `App` that assembles from IoT config + new fields
- [x] 2.4 Compile and verify

## Phase 3: Add WorkerCommand Variants
- [x] 3.1 Add `DeviceProvision`, `DeviceSetup`, `DeviceUpdate`, `DeviceStart`, `DeviceStop`, `DeviceStatus`, `DeviceBackup`, `DeviceRestore` to `WorkerCommand`
- [x] 3.2 Wire each variant to `IoTDeployer` methods in `worker.rs`
- [x] 3.3 Add `WorkerResponse::ConfirmationNeeded` for destructive operations requiring user confirmation
- [x] 3.4 Compile and verify

## Phase 4: Restructure TUI Tabs
- [x] 4.1 Add `Device` tab to the `Tab` enum (between IoTConfig and Config)
- [x] 4.2 Move existing device-oriented actions from Actions tab to Device tab (SyncTime moved to Device)
- [x] 4.3 Actions tab now has: GenerateConfig, SendConfig, ClearMessages, TelegrafStatus, TelegrafLogs, RestartTelegraf, ServiceStatus, BackupGrafana
- [x] 4.4 Add new Device tab actions: Provision, Setup, Update, Start, Stop, Status, Backup, Restore, SyncTime, DeviceName, KeyFile, GitBranch, Minimal, LocalTransfer
- [x] 4.5 Render the Device tab with selection + hotkeys
- [x] 4.6 Handle Device tab input (including confirmation flow for Stop)
- [x] 4.7 Compile and verify

## Phase 5: Confirmation Flow
- [x] 5.1 Render confirmation overlay when `pending_confirmation` is `Some`
- [x] 5.2 On y/N input, dispatch or cancel the pending command (global handler in event loop)
- [x] 5.3 Wire destructive operations (Stop, Stop+Volumes) through confirmation
- [x] 5.4 Compile and verify

## Phase 6: Long-Running Operation Progress
- [x] 6.1 Add `is_working` visual indicator in the tab bar title
- [x] 6.2 Confirmation overlay shown with ⚠️ indicator in tab bar
- [x] 6.3 Compile and verify

## Phase 7: Polish
- [x] 7.1 Update help popup with new tab and keybindings
- [x] 7.2 Status messages for all device operations
- [x] 7.3 Final compile and test