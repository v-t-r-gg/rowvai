#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(commands::AppState::default())
        .setup(|app| {
            let _ = app.path().app_data_dir();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::new_workbook,
            commands::create_table,
            commands::add_column,
            commands::get_grid_data,
            commands::insert_row,
            commands::open_workbook,
            commands::get_tables,
            commands::update_cell,
            commands::delete_row_cmd,
            commands::list_approval_inbox,
            commands::get_approval_detail,
            commands::approve_and_execute_operation,
            commands::reject_operation,
            commands::revise_operation,
            commands::preview_operation_undo,
            commands::commit_operation_undo,
            commands::list_operation_history,
            commands::get_operation_detail,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
