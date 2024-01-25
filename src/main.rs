use std::fs::{self, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write};
use std::path::Path;
use std::process::Command;

use ips::Patch;

const DATA_FILES: [&str; 2] = ["./temp/SLUS_014.11", "./temp/DATA/WA_MRG.MRG"];
const PATCH_FILES: [&str; 2] = ["SLUS_014.ips", "WA_MRG.ips"];

fn apply_patch(target_file: &Path, patch_contents: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut rom = OpenOptions::new().write(true).open(target_file)?;
    let patch = Patch::parse(&patch_contents)?;

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


    println!("\
    How many cards should drop after each duel victory?
    Enter an integer greater than 1.
    Values up to 15 (inclusive) are supported.
    Values greater than 15 may cause issues.");

    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Failed to read line");
    let user_input: u32 = match input.trim().parse() {
        Ok(num) => num,
        Err(_) => {
            println!("Invalid input. Please enter a valid positive integer.");
            return Ok(());
        }
    };
    if user_input <= 1 || user_input > 254 {
        println!("Values less than 2 or greater than 254 cannot be applied.");
        return Ok(());
    }
    let fixed_input = user_input + 1; // 0x06 means 5 cards drop

    let value_as_byte: u8 = fixed_input as u8;
    let adjust_number_of_drops_patch: [u8; 14] = [
    	0x50, 0x41, 0x54, 0x43, 0x48, 0xBC, 0x17, 0xE4, 0x00, 0x01, value_as_byte, 0x45,
    	0x4F, 0x46
    ];

    for (target_file, patch_file) in DATA_FILES.iter().zip(PATCH_FILES.iter()) {
        let target_path = Path::new(&target_file);
        let concat = format!("./patches/{}", patch_file);
        let patch_path = Path::new(&concat);
        let patch_contents = fs::read(&patch_path)?;
        apply_patch(target_path, &patch_contents)?;
    }
    let wa_mrg_path = Path::new(DATA_FILES[1]);
    apply_patch(wa_mrg_path, &adjust_number_of_drops_patch)?;
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
