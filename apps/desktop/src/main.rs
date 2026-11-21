// Sem janela de consola no Windows em modo release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tasca_app_lib::run()
}
