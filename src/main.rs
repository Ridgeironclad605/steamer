use std::sync::Arc;

use indicatif::MultiProgress;
use steamer::{AssetType, SteamGridClient};
use steamlocate::Shortcut;

struct MockShortcut {
    app_name: String,
    app_id: u32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("STEAMGRID_API_KEY").expect("STEAMGRID_API_KEY must be set");
    let client = SteamGridClient::new(&api_key)?;

    let steam = steamlocate::locate()?;
    // let user_id: u64 = {
    //     let login_users_vdf = steam.path().join("config").join("loginusers.vdf");
    //     let contents = std::fs::read_to_string(login_users_vdf)?;
    //     let obj = keyvalues_parser::Vdf::parse(&contents)?.value.unwrap_obj();
    //     obj.keys().next().unwrap().parse::<u64>()? - 76561197960265728
    // };

    let grid_base = std::path::PathBuf::from("./test_grid");

    // let grid_base = steam
    //     .path()
    //     .join("userdata")
    //     .join(user_id.to_string())
    //     .join("config")
    //     .join("grid");

    std::fs::create_dir_all(&grid_base)?;

    println!("Using Steam directory - {}", steam.path().display());

    // Non steam games
    // let shortcuts = steam
    //     .shortcuts()?
    //     .filter_map(Result::ok)
    //     .collect::<Vec<Shortcut>>();

    let shortcuts = vec![
        MockShortcut {
            app_name: "Hades".into(),
            app_id: 1,
        },
        MockShortcut {
            app_name: "Celeste".into(),
            app_id: 2,
        },
        MockShortcut {
            app_name: "Dead Cells".into(),
            app_id: 3,
        },
    ];

    println!("Found {} non-steam game(s)!\n", shortcuts.len());

    for s in shortcuts {
        let games = client.search_by_name(&s.app_name).await?;

        let Some(game) = games.first() else {
            println!("No match for {}\n", s.app_name);
            continue;
        };

        println!("Downloading assets for: {} (app_id {})", game.name, game.id);

        let (grids, heroes, logos) = tokio::join!(
            client.find_asset(game.id, AssetType::Grid),
            client.find_asset(game.id, AssetType::Hero),
            client.find_asset(game.id, AssetType::Logo),
        );

        let grids = grids?;
        let heroes = heroes?;
        let logos = logos?;

        let mp = Arc::new(MultiProgress::new());

        let (grid, hero, logo) = tokio::join!(
            client.download_asset(&grids[0], AssetType::Grid, mp.clone()),
            client.download_asset(&heroes[0], AssetType::Hero, mp.clone()),
            client.download_asset(&logos[0], AssetType::Logo, mp)
        );

        grid?.save(s.app_id, &grid_base, AssetType::Grid)?;
        hero?.save(s.app_id, &grid_base, AssetType::Hero)?;
        logo?.save(s.app_id, &grid_base, AssetType::Logo)?;

        println!("\n\n");
    }

    println!("Done! All assets were saved at {}", grid_base.display());

    Ok(())
}
