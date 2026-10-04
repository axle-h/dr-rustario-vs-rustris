//! Draws the pills where a trained Dr. Rustario model cleared nothing with a kill on offer and no
//! combo followed: the bottle before, then the chosen placement and the kill it passed up, each
//! as it lands and after it settles, with `manifest.json` holding every input of both and how
//! far each one pulled the network away from the kill ([`dr_rustario::game::ai::passes::oddities`]).
//!
//! `cargo run --release -p dr-rustario-vs-rustris --example oddity_shots -- out/ weights [count] [theme]`

#[path = "common/bottle_shots.rs"]
mod bottle_shots;

use bottle_shots::Shooter;
use dr_rustario::game::ai::explain::INPUTS;
use dr_rustario::game::ai::genetic::load_weights;
use dr_rustario::game::ai::passes::oddities;

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).cloned().unwrap_or_else(|| ".".to_string());
    let weights = args.get(2).ok_or("which weights?")?;
    let count: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(8);
    let theme = args.get(4).cloned().unwrap_or_else(|| "nes".to_string());

    let network = load_weights(weights)?.into();
    let found = oddities(network, count);
    let mut shooter = Shooter::new("oddity-shots", &theme)?;

    let mut manifest = vec![];
    for (at, oddity) in found.iter().enumerate() {
        let drawn = &oddity.drawn;
        let shots = [
            ("before", &drawn.before, None, [].as_slice()),
            (
                "chosen-landed",
                &drawn.landed[0],
                Some(drawn.placed[0]),
                drawn.destroyed[0].as_slice(),
            ),
            ("chosen-after", &drawn.after[0], None, [].as_slice()),
            (
                "kill-landed",
                &drawn.landed[1],
                Some(drawn.placed[1]),
                drawn.destroyed[1].as_slice(),
            ),
            ("kill-after", &drawn.after[1], None, [].as_slice()),
        ];
        for (what, bottle, placed, destroyed) in shots {
            shooter.shoot(
                bottle,
                placed,
                destroyed,
                &format!("{out}/{at:02}-{what}.png"),
            )?;
        }
        let inputs: Vec<String> = INPUTS
            .iter()
            .enumerate()
            .map(|(input, about)| {
                format!(
                    r#"{{"name": "{}", "chosen": {}, "kill": {}, "pull": {:.4}}}"#,
                    about.name, oddity.raw[0][input], oddity.raw[1][input], oddity.pulls[input]
                )
            })
            .collect();
        manifest.push(format!(
            r#"  {{"at": {at}, "game": {}, "level": {}, "bottle": {}, "pill": {}, "viruses": {},
   "kills": {}, "margin": {:.4}, "viruses_after": {}, "pills_after": {}, "kill_rounds": {},
   "inputs": [{}]}}"#,
            oddity.game,
            oddity.level,
            oddity.bottle_number,
            oddity.pill,
            oddity.viruses,
            oddity.kills,
            oddity.margin,
            oddity.viruses_after,
            oddity.pills_after,
            drawn.rounds[1],
            inputs.join(", ")
        ));
        println!(
            "{at:02}: game {} bottle {} pill {}, passed up {} viruses by {:.3}",
            oddity.game, oddity.bottle_number, oddity.pill, oddity.kills, oddity.margin
        );
    }
    let path = format!("{out}/manifest.json");
    std::fs::write(&path, format!("[\n{}\n]\n", manifest.join(",\n")))
        .map_err(|e| e.to_string())?;
    println!("{} oddities and {path}", found.len());
    Ok(())
}
