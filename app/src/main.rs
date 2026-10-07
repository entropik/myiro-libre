// Pas de console derrière la fenêtre dans la version publiée.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    app::lancer();
}
