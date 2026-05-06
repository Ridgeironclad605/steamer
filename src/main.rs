use std::sync::Arc;

use indicatif::MultiProgress;
use new_vdf_parser::open_shortcuts_vdf;
use new_vdf_parser::write_shortcuts_vdf;
use serde_json::Map;
use serde_json::Value;
use steamer::AssetType;
use steamer::SteamGridClient;
use steamer::SteamPaths;
use steamer::choose_game;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("STEAMGRID_API_KEY").expect("STEAMGRID_API_KEY must be set");
    let client = SteamGridClient::new(&api_key)?;

    let interactive = std::env::args().any(|a| a == "--interactive");

    let steam = steamlocate::locate()?;
    println!("Found Steam directory - {}", steam.path().display());

    let steam_paths = SteamPaths::locate(&steam)?;

    println!("Using Grid directory - {}", steam_paths.grid.display());
    std::fs::create_dir_all(&steam_paths.grid)?;

    let mut shortcuts_vdf = open_shortcuts_vdf(&steam_paths.shortcuts);

    let shortcuts = shortcuts_vdf
        .as_object_mut()
        .expect("shortcuts_vdf must be a json object");

    println!("Found {} non-steam game(s)!\n", shortcuts.len());

    for (_, v) in shortcuts.iter_mut() {
        let app_name = v["AppName"].as_str().unwrap();
        let app_id = v["appid"].as_u64().unwrap() as u32;

        let games = client.search_by_name(app_name).await?;

        let Some(game) = choose_game(&games, interactive) else {
            println!("No match for {app_name}\n");
            continue;
        };

        println!("Downloading assets for: {} (app_id {})", game.name, game.id);

        let (grids, heroes, logos, icons) = tokio::join!(
            client.find_asset(game.id, AssetType::Grid),
            client.find_asset(game.id, AssetType::Hero),
            client.find_asset(game.id, AssetType::Logo),
            client.find_asset(game.id, AssetType::Icon),
        );

        let grids = grids?;
        let heroes = heroes?;
        let logos = logos?;
        let icons = icons?;

        if grids.is_empty() || heroes.is_empty() || logos.is_empty() || icons.is_empty() {
            println!("Not enough assets for this game, skipping...\n");
            continue;
        }

        let mp = Arc::new(MultiProgress::new());

        let (grid, hero, logo, icon) = tokio::join!(
            client.download_asset(&grids[0], AssetType::Grid, mp.clone()),
            client.download_asset(&heroes[0], AssetType::Hero, mp.clone()),
            client.download_asset(&logos[0], AssetType::Logo, mp.clone()),
            client.download_asset(&icons[0], AssetType::Icon, mp)
        );

        grid?.save(app_id, &steam_paths.grid, AssetType::Grid)?;
        hero?.save(app_id, &steam_paths.grid, AssetType::Hero)?;
        logo?.save(app_id, &steam_paths.grid, AssetType::Logo)?;

        // Icons need to be updated in the vdf
        let icon_path = icon?.save(app_id, &steam_paths.grid, AssetType::Icon)?;
        v["icon"] = Value::String(icon_path);

        println!("\n\n");
    }

    let mut vdf_to_write = Value::Object(Map::new());
    vdf_to_write["shortcuts"] = shortcuts_vdf;

    write_shortcuts_vdf(&steam_paths.shortcuts, vdf_to_write);
    println!(
        "Done! All assets were saved at {}",
        steam_paths.grid.display()
    );

    Ok(())
}
