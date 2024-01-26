use std::fs::{self, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write};
use std::path::Path;
use std::process::Command;

use ips::Patch;

fn apply_patch(target_file: &Path, patch_file: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut rom = OpenOptions::new().write(true).open(target_file)?;
    let patch_open = fs::read(patch_file)?;
    let patch = Patch::parse(&patch_open)?;

    for hunk in patch.hunks() {
        rom.seek(SeekFrom::Start(hunk.offset() as u64))?;
        rom.write_all(hunk.payload())?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    //extract FM disc
    let bin_name = "Yu-Gi-Oh! Forbidden Memories (USA).bin";
    if fs::metadata(bin_name).is_ok() {
        println!("Found {}", bin_name);
    } else {
        eprintln!("Could not find {}. Copy the file in the same directory as this executable and try again.", bin_name);
        return Ok(());
    }
    let temp_dir = "temp";
    if let Err(err) = fs::create_dir(temp_dir) {
        if err.kind() != std::io::ErrorKind::AlreadyExists {
            eprintln!("Error creating temporary extraction directory: {}", err);
            return Ok(());
        }
    } else {
        println!("Created temporary directory \"temp\"");
    }
    let dumpsxiso_path = Path::new("./bin/dumpsxiso");
    let extract_command = Command::new(&dumpsxiso_path)
        .arg("-x")
        .arg("temp")
        .arg("-s")
        .arg("YGO.xml")
        .arg(bin_name)
        .output()
        .expect("Error running PSX BIN extractor");
    io::stdout().write_all(&extract_command.stdout).unwrap();
    io::stderr().write_all(&extract_command.stderr).unwrap();


    println!("How many cards should drop after each duel victory?");
    println!("Integer values between 2 and 15 (inclusive) are supported.");

    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Failed to read line");
    let user_input: i32 = match input.trim().parse() {
        Ok(num) => num,
        Err(_) => {
            println!("Invalid input. Please enter a valid positive integer.");
            return Ok(());
        }
    };
    if user_input <= 1 || user_input > 15 {
        println!("Values less than 2 or greater than 15 cannot be applied.");
        return Ok(());
    }
    println!("Patching SLUS_014.11");
    let slus_path = Path::new("./temp/SLUS_014.11");
    let patch_for_slus_path = Path::new("./data_patches/SLUS_014.ips");
    apply_patch(slus_path, patch_for_slus_path)?;

    println!("Patching WA_MRG.MRG");
    let wa_path = Path::new("./temp/DATA/WA_MRG.MRG");
    let formatted = format!("./data_patches/{}card.ips", user_input);
    let patch_for_wa_path = Path::new(&formatted);
    apply_patch(wa_path, patch_for_wa_path)?;

    println!("Patched as {}-card mod", user_input);

    println!("Rebuilding ROM as YGOFM-{}CardMod.bin/cue", user_input);
    let mkpsxiso_path = Path::new("./bin/mkpsxiso");
    let rebuild_command = Command::new(&mkpsxiso_path)
        .arg("-o")
        .arg(format!("YGOFM-{}CardMod.bin", user_input))
        .arg("YGO.xml")
        .output()
        .expect("Error running mkpsxiso");
    io::stdout().write_all(&rebuild_command.stdout).unwrap();
    io::stderr().write_all(&rebuild_command.stderr).unwrap();
    // mkpsxiso supports renaming the bin but not the cue
    if let Err(err) = fs::rename("mkpsxiso.cue", format!("YGOFM-{}CardMod.cue", user_input)) {
        eprintln!("Error renaming .cue file: {}", err);
    }
    println!("Success. You may now delete the temp directory.");
    Ok(())
}
