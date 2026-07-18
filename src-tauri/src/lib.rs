#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod storage;
mod engine;
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
])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}