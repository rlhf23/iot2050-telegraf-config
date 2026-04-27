# TUI Device Lifecycle Integration Plan

## Goal
Add device lifecycle management (provision, setup, update, start, stop, status, backup, restore) to the TUI, and fix the event loop to support long-running operations.

## Phase 1: Fix Event Loop (Prerequisites)
- [x] 1.1 Switch from blocking `event::read()` to `event::poll(timeout)` in `run_app` loop
- [x] 1.2 Change `process_worker_responses` to drain all responses per cycle (not just one)
- [x] 1.3 Compile and verify

## Phase 2: Add DeploymentConfig to App State
- [x] 2.1 Add `device_name`, `key_file`, `git_branch` fields to the IoT Config tab in `App`
- [x] 2.2 Add `minimal` and `local_transfer` flags to `App` for device operations
- [x] 2.3 Create a `DeploymentConfig` builder method on `App` that assembles from IoT config + new fields
- [x] 2.4 Compile and verify

## Phase 3: Add WorkerCommand Variants
- [ ] 3.1 Add `DeviceProvision`, `DeviceSetup`, `DeviceUpdate`, `DeviceStart`, `DeviceStop`, `DeviceStatus`, `DeviceBackup`, `DeviceRestore` to `WorkerCommand`
- [ ] 3.2 Wire each variant to `IoTDeployer` methods in `worker.rs`
- [ ] 3.3 Add `WorkerResponse::ConfirmationNeeded` for destructive operations requiring user confirmation
- [ ] 3.4 Compile and verify

## Phase 4: Restructure TUI Tabs
- [ ] 4.1 Add `Device` tab to the `Tab` enum (between Config and Actions, or replacing Actions)
- [ ] 4.2 Move existing device-oriented actions from Actions tab to Device tab (SyncTime, check_service_status becomes ServiceStatus)
- [ ] 4.3 Rename Actions tab to only contain config-generation actions (GenerateConfig, SendConfig, ClearMessages)
- [ ] 4.4 Add new Device tab actions: Provision, Setup, Update, Start, Stop, Status, Backup, Restore
- [ ] 4.5 Render the Device tab with selection + hotkeys
- [ ] 4.6 Handle Device tab input
- [ ] 4.7 Compile and verify

## Phase 5: Confirmation Flow
- [ ] 5.1 Add `pending_confirmation: Option<WorkerCommand>` field to `App`
- [ ] 5.2 Render confirmation overlay when `pending_confirmation` is `Some`
- [ ] 5.3 On y/N input, dispatch or cancel the pending command
- [ ] 5.4 Wire destructive operations (Stop with volumes, Restore with force) through confirmation
- [ ] 5.5 Compile and verify

## Phase 6: Long-Running Operation Progress
- [ ] 6.1 Add `is_working` visual indicator (spinner or "Working..." text) in the UI
- [ ] 6.2 Ensure provision/update/setup send `ProgressUpdate` responses through the worker
- [ ] 6.3 Verify progress messages flow through to the status area during long operations
- [ ] 6.4 Compile and manual test

## Phase 7: Polish
- [ ] 7.1 Update help popup with new tab and keybindings
- [ ] 7.2 Ensure all status messages are user-friendly for device operations
- [ ] 7.3 Final compile and test