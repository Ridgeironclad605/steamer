use std::io;
use std::io::Write;
use std::sync::Arc;

use comfy_table::Table;
use indicatif::MultiProgress;
use new_vdf_parser::open_shortcuts_vdf;
use new_vdf_parser::write_shortcuts_vdf;
use serde_json::Map;
use serde_json::Value;
use steamer::AssetType;
use steamer::GameSearchObject;
use steamer::SteamGridClient;
use steamlocate::Shortcut;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("STEAMGRID_API_KEY").expect("STEAMGRID_API_KEY must be set");
    let client = SteamGridClient::new(&api_key)?;

    let interactive = std::env::args().any(|a| a == "--interactive");

    let steam = steamlocate::locate()?;
    let user_id: u64 = {
        let login_users_vdf = steam.path().join("config").join("loginusers.vdf");
        let contents = std::fs::read_to_string(login_users_vdf)?;
        let obj = keyvalues_parser::Vdf::parse(&contents)?.value.unwrap_obj();
        obj.keys().next().unwrap().parse::<u64>()? - 76561197960265728
    };

    // I hate that I needed an entire libary for this
    let shortcuts_vdf_path = steam
        .path()
        .join("userdata")
        .join(user_id.to_string())
        .join("config")
        .join("shortcuts.vdf");

    let mut shortcut_vdf = open_shortcuts_vdf(&shortcuts_vdf_path);

    let grid_base = steam
        .path()
        .join("userdata")
        .join(user_id.to_string())
        .join("config")
        .join("grid");

    std::fs::create_dir_all(&grid_base)?;

    println!("Found Steam directory - {}", steam.path().display());

    // Non steam games
    let shortcuts = steam
        .shortcuts()?
        .filter_map(Result::ok)
        .collect::<Vec<Shortcut>>();

    println!("Found {} non-steam game(s)!\n", shortcuts.len());

    for (i, s) in shortcuts.iter().enumerate() {
        let games = client.search_by_name(&s.app_name).await?;

        let Some(game) = choose_game(&games, interactive) else {
            println!("No match for {}\n", s.app_name);
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

        grid?.save(s.app_id, &grid_base, AssetType::Grid)?;
        hero?.save(s.app_id, &grid_base, AssetType::Hero)?;
        logo?.save(s.app_id, &grid_base, AssetType::Logo)?;

        // Icons need to be updated in the vdf
        let icon_path = icon?.save(s.app_id, &grid_base, AssetType::Icon)?;
        shortcut_vdf[i.to_string()]["icon"] = Value::String(icon_path);

        println!("\n\n");
    }

    let mut actual_vdf = Value::Object(Map::new());
    actual_vdf["shortcuts"] = shortcut_vdf;

    write_shortcuts_vdf(&shortcuts_vdf_path, actual_vdf);
    println!("Done! All assets were saved at {}", grid_base.display());

    Ok(())
}

fn choose_game(games: &'_ [GameSearchObject], interactive: bool) -> Option<&'_ GameSearchObject> {
    if !interactive || games.is_empty() {
        return games.first();
    }

    let mut table = Table::new();
    table.set_header(vec!["#", "Name", "ID"]);

    let max_choices = games.len().min(5);

    // Only show the first 5 games, others are almost always irrelevant
    (0..max_choices).for_each(|i| {
        table.add_row(&[
            i.to_string(),
            games[i].name.to_string(),
            games[i].id.to_string(),
        ]);
    });

    println!("Choose which game to pick:\n{table}");

    games.get(read_choice(max_choices))
}

fn read_choice(max: usize) -> usize {
    loop {
        print!("Enter choice (0-{}): ", max - 1);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        if let Ok(n) = input.trim().parse::<usize>()
            && n < max
        {
            return n;
        }

        println!("Invalid choice, try again.");
    }
}
