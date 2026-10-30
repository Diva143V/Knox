//! Knox Desktop binary entry point.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    knox_desktop_lib::run();
}
