mod random_level_worker;
mod random;

use fancy_regex::{Regex, SubCaptureMatches};

use rusqlite::{Connection, Result};

use serde::{Deserialize, Serialize};
use serde_json;

use snap;
use snap::raw::{Decoder, Encoder};

use core::time;
use std::collections::HashMap;
use std::fs::{self, create_dir, Metadata};
use std::time::SystemTime;

/**
 * Data struct stores the savedGame and settings of the world
 */
#[derive(Serialize, Deserialize, Debug)]
pub struct Data {
    pub js_level: JSLevel,
    pub settings: Settings
}

impl Data {
    pub fn new (js_level: JSLevel, settings: Settings) -> Self {
        Data {js_level, settings}
    }
}


/**
 * JSLevel struct stores the object format of a
 * classic js level of type:
 * {"worldSeed":0,"changedBlocks":{},"worldSize":128,"version":1}
 * References the ChangedBlocks struct
 * worldSeed: This is the seed of the world
 * changedBlocks: This is an array of all changedBlocks in the world
 * worldSize: This is the width/length of the world, must be 128, 256, or 512
 * version: Yeah, I have no clue what this is, but it's seemingly always 1 so...
 */
#[derive(Serialize, Deserialize, Debug)]
pub struct JSLevel {
    pub worldSeed: i64,
    pub changedBlocks: HashMap<String,ChangedBlocks>,
    pub worldSize: i32,
    pub version: u8
}

impl JSLevel {
    pub fn new (worldSeed: i64, changedBlocks: HashMap<String,ChangedBlocks>, worldSize: i32, version: u8) -> Self {
        JSLevel { worldSeed, changedBlocks, worldSize, version } 
    }

    pub fn default () -> Self {
        JSLevel { worldSeed: 1, changedBlocks: HashMap::new(), worldSize: 256, version: 1 }
    }
}

/**
 * ChangedBlocks struct stores the json object of type:
 * p0_0_0: {a: 0, bt: 0}
 * This object is used inside the savedGame object to keep track
 * of each changed block in the world:
 * p0_0_0: position of block in world, where px_y_z
 * a: 0 if block does match natural generation / 1 if block does not match natural generation
 * bt: type of block
 */
#[derive(Serialize, Deserialize, Debug)]
pub struct ChangedBlocks {pub a: u8, pub bt: u8}
impl ChangedBlocks { pub fn new (a: u8, bt: u8) -> Self {ChangedBlocks { a, bt }}}

/**
 * Settings struct stores the json object containing all settings for javascript worlds
 * These settings include typical control and sound settings, but they also contain the username
 */
#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    pub music: bool,
    pub sound: bool,
    pub invert: bool,
    pub fps: bool,
    pub drawDistance: i32,
    pub forward: String,
    pub left: String,
    pub backward: String,
    pub right: String,
    pub jump: String,
    pub build: String,
    pub chat: String,
    pub fog: String,
    pub saveLoc: String,
    pub loadLoc: String,
    pub username: String
}

impl Settings {
    pub fn new(
        music: bool,
        sound: bool,
        invert: bool,
        fps: bool,
        drawDistance: i32,
        forward: String,
        left: String,
        backward: String,
        right: String,
        jump: String,
        build: String,
        chat: String,
        fog: String,
        saveLoc: String,
        loadLoc: String,
        username: String
    ) -> Self {
        Settings { music, sound, invert, fps, drawDistance, forward, left, backward, right, jump, build, chat, fog, saveLoc, loadLoc, username }
    }

    pub fn default () -> Self {
        Settings {
            music: false,
            sound: true,
            invert: false,
            fps: false,
            drawDistance: 0,
            forward: String::from("W"),
            left: String::from("A"),
            backward: String::from("S"),
            right: String::from("D"),
            jump: String::from("<space>"),
            build: String::from("B"),
            chat: String::from("T"),
            fog: String::from("F"),
            saveLoc: String::from("<enter>"),
            loadLoc: String::from("R"),
            username: String::from("noname")
        }
    }
}

/**
 * LocalStorage struct stores input from localStorage db files
 * key: "savedGame"
 * utf16_length: Length of uncompressed value
 * conversion_type: 1
 * compression_type: 1
 * value: The actual savedGame, so the actual world
 */
pub struct LocalStorage {
    key: String,
    utf16_length: i32,
    conversion_type: i32,
    compression_type: i32,
    last_access_time: i32,
    value: Vec<u8>
}

/**
 * Converts a json string in the savedGame format into
 * a JSLevel struct
 */
pub fn deserialize_saved_game (json_string: String) -> JSLevel {
    let level: JSLevel = serde_json::from_str(&json_string).unwrap();
    return level;
}

/**
 * Converts a json string in the settings format into
 * a Settings struct
 */
pub fn deserialize_settings (json_string: String) -> Settings {
    let settings: Settings = serde_json::from_str(&json_string).unwrap();
    return settings;
}

/**
 * Converts a savedGame json string and a settings json string
 * into a Data struct
 */
pub fn deserialize_data (json_string1: String, json_string2: String) -> Data {
    let level: JSLevel = serde_json::from_str(&json_string1).unwrap();
    let settings: Settings = serde_json::from_str(&json_string2).unwrap();
    return Data { js_level: level, settings: settings}
}

/**
 * Following function accepts a level in the JS form, a tile_map, and optimization and
 * writes it into the classic javascript object format
 */
pub fn serialize_saved_game (level: JSLevel, tile_map: Vec<u8>, opt: u8) -> String {

    //Assigning x, y, and z of world
    let x: i32 = level.worldSize;
    let y: i32 = 64;
    let z: i32 = level.worldSize;
    let tile_map1 = get_tile_map(level.worldSize, level.worldSeed);

    let mut output: String = String::from("{"); //Opening json object

    output += &format!(r#""worldSeed":{},"#,level.worldSeed.to_string()); //Adding seed key value pair

    //Adding changed blocks key value pair
    output += r#""changedBlocks":"#; //Adding blocks key
    output += "{"; //Opening block values object

    //Variables for the tiles and a value
    let mut t: u8;
    let mut t1: u8;
    let mut a: u8; //a = 0 if changed block matches generation, a = 1 if changed block does not match generation

    //Iterating through all blocks
    //Tilemaps are stored in X,Z,Y format, where [0] is X:0, Y:0, Z:0 & [1] is X:1, Y:0, Z:0 etc.
    let mut flag: bool = false;
    for i in 0..y {
        for j in 0..z {
            for k in 0..x {

                /* Following code block will be more useful once a changed blocks hashmap is implemented */

                //Setting tile for changed block and checking whether it matches tile generated by seed
                let mut flag1 = false;
                let key: String = String::from(format!(r#"p{}_{}_{}"#,k,i,j));
                //Grabbing the block directly from level
                let bt: u8 = level.changedBlocks.get(&key).unwrap_or(&ChangedBlocks::new(1,255)).bt;
                //Grabbing block from passed in tile map
                t = tile_map[((i*z*x) + (j*x) + k) as usize];
                //Grabbing the block generated from world
                t1 = tile_map1[((i*z*x) + (j*x) + k) as usize];
                if bt != 255 { t = bt }
                if t == t1 { a = 0 } else { a = 1 } //a = 0 if changed block matches generation, a = 1 if changed block does not match generation

                //If opt == 2 the tile must differ from natural generation to write to array
                //If opt == 1 either the tile differs from natural generation or it is already considered a changed block to write to array
                //If opt == 0 tile is written to array
                //Default value should be 1 or 2, opt 0 is storage intensive and causes unnecessary lag
                if (opt == 2 && a == 1) || (opt == 1 && (bt != 255 || a == 1)) || opt == 0 { flag1 = true }
                
                if flag1 {
                    //Creating key for changed block
                    output += &key;

                    //Creating value for changed block
                    output += "{";
                    output += &format!(r#""a":{},"bt":{}"#,a,t);
                    output += "},";

                    flag = true;
                }

            }
        }
    }

    if flag {output.pop();} //Removing extra comma
    output += "},"; //Closing Changed Blocks object

    output += &format!{r#""worldSize":{},"#,level.worldSize}; //Adding world size key value pair
    output += &format!{r#""version":{}"#,level.version}; //Adding version key value pair

    output += "}"; //Closing json object
    return output;

}

/**
 * Following function accepts a settings object and returns 
 * a serialized json string
 */
pub fn serialize_settings (settings: Settings) -> String {
    let mut output: String = String::from("{"); //Opening json object
    output += &format!{r#""music":{},"#,settings.music};
    output += &format!{r#""sound":{},"#,settings.sound};
    output += &format!{r#""invert":{},"#,settings.invert};
    output += &format!{r#""fps":{},"#,settings.fps};
    output += &format!{r#""drawDistance":{},"#,settings.drawDistance};
    output += &format!{r#""forward":"{}","#,settings.forward};
    output += &format!{r#""left":"{}","#,settings.left};
    output += &format!{r#""backward":"{}","#,settings.backward};
    output += &format!{r#""right":"{}","#,settings.right};
    output += &format!{r#""jump":"{}","#,settings.jump};
    output += &format!{r#""build":"{}","#,settings.build};
    output += &format!{r#""chat":"{}","#,settings.chat};
    output += &format!{r#""fog":"{}","#,settings.fog};
    output += &format!{r#""saveLoc":"{}","#,settings.saveLoc};
    output += &format!{r#""loadLoc":"{}","#,settings.loadLoc};
    output += &format!{r#""username":"{}""#,settings.username};
    output += "}"; //Closing json object
    return output;
}

/**
 * Follwing function accepts a Data struct and returns two serialized json
 * strings
 */
pub fn serialize_data (data: Data) -> [String; 2] {
    let tile_map = get_tile_map(data.js_level.worldSize, data.js_level.worldSeed);
    let level_str: String = serialize_saved_game(data.js_level, tile_map, 1);
    let settings_str: String = serialize_settings(data.settings);
    return [level_str, settings_str]
}

/**
 * Following function opens an sqlite database at the provided path,
 * then retreives the specified object, and then decompresses it 
 * before returning it
 */
pub fn read_from_db (file_path: String, object: &str) -> Result<String> {

    let conn: Connection = Connection::open(file_path)?;

    let mut stmt = conn.prepare(
        "SELECT * FROM data where key=?1;"
    )?;

    //Iterating through the database
    let entries = stmt.query_map([object], |row| Ok(
        LocalStorage {
            key: row.get(0)?,
            utf16_length: row.get(1)?,
            conversion_type: row.get(2)?,
            compression_type: row.get(3)?,
            last_access_time: row.get(4)?,
            value: row.get(5)?,
        }
    ))?;

    //Retreiving the compressed save game object and length
    let mut compressed_object: Vec<u8> = Vec::new();
    let mut decompressed_length: i32 = 0;
    for entry in entries {
        let local: LocalStorage = entry.unwrap();
        if local.key == object {
            compressed_object = local.value;
            decompressed_length = local.utf16_length;
            break;
        }
    }

    //Creating an array with the correct length for storing the decompressed bytes
    let mut decompressed: Vec<u8> = Vec::new();
    for _ in 0..decompressed_length {
        decompressed.push(0);
    }

    //Decompressing using snappy compression
    Decoder::decompress(&mut Decoder::new(), &compressed_object, &mut decompressed).unwrap();

    //Converting the character codes to characters
    let mut characters: Vec<char> = Vec::new();
    for ch in decompressed {
        characters.push(ch as char)
    }

    //Returning the characters as a string
    Ok(characters.iter().collect())

}

/**
 * Following function opens an sqlite database at the provided path,
 * then retreives the specified object, and then decompresses it 
 * before returning it
 */
pub fn read_saved_game (file_path: String) -> Result<String> {
    return read_from_db(file_path, "savedGame");
}

/**
 * Following function opens an sqlite database at the provided path,
 * then retreives the specified object, and then decompresses it 
 * before returning it
 */
pub fn read_settings (file_path: String) -> Result<String> {
    return read_from_db(file_path, "settings");
}

/**
 * Following function accepts a path to a db file, and a 
 * json string. The json string is parsed as the value and
 * compressed using snappy compression, and is then passed
 * to the db and saved. Note this only applies to Firefox,
 * as firefox is the only browser that I know of that uses
 * this structure. Chromium support in the future...
 */
pub fn write_data (file_path: String, json_strings: [String; 2], website: String) -> Result<()> {

    let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_micros() as u64;

    //Creating directories
    let regex = Regex::new(r#"/|:|\*|\?|"|>|<|\||\\"#).unwrap();
    let substitution = "+";
    let dir_name = regex.replace_all(&website, substitution);

    create_dir(file_path.clone() + "/" + &dir_name);
    create_dir(file_path.clone() + "/" + &dir_name + "/ls");

    //Building metadata file
    let mut metadata: Vec<u8> = Vec::new();
    metadata.extend_from_slice(&timestamp.to_be_bytes()); //Timestamp
    metadata.push(0); //Persisted
    metadata.extend_from_slice(&(0 as i32).to_be_bytes()); //Suffix
    metadata.extend_from_slice(&(0 as i32).to_be_bytes()); //Group

    //Origin
    metadata.extend_from_slice(&(website.len() as u16).to_be_bytes());
    let chars: Vec<char> = website.chars().collect();
    for ch in chars {metadata.push(ch as u8)}

    metadata.push(0); //Is App

    fs::write(file_path.clone() + "/" + &dir_name + "/.metadata-v2", metadata);

    let keys: Vec<&str> = vec!["savedGame", "settings"];

    let conn: Connection = Connection::open(file_path.clone() + "/" + &dir_name + "/ls/data.sqlite")?;

    conn.pragma_update(None, "user_version", 80);
    conn.pragma_update(None, "auto_vacuum", 2);
    conn.pragma_update(None, "page_size", 1024);

    conn.execute("VACUUM", []);

    //Creates the localStorage data table inside the database if it does not exist
    conn.execute(
        "CREATE TABLE if not exists data ( 
        key TEXT PRIMARY KEY, 
        utf16_length INTEGER NOT NULL, 
        conversion_type INTEGER NOT NULL, 
        compression_type INTEGER NOT NULL, 
        last_access_time INTEGER NOT NULL DEFAULT 0, 
        value BLOB NOT NULL)", 
        []
    )?;

    let mut len = 0;

    //Inserting the savedGame into the database
    let mut stmt = conn.prepare("INSERT OR REPLACE INTO data (key, utf16_length, conversion_type, compression_type, value) values (?1, ?2, ?3, ?4, ?5)" )?;

    for i in 0..json_strings.len() {
        //Converting the json_string into an array of chars
        let characters: Vec<char> = json_strings[i].chars().collect();
        let utf16_length: i32  = characters.len() as i32;

        len += utf16_length;

        //Converting chars to u8
        let mut decompressed: Vec<u8> = Vec::new();
        for ch in characters {
            decompressed.push(ch as u8);
        }

        //Creating the output array
        let max_comp_length = snap::raw::max_compress_len(decompressed.len());
        let mut compressed: Vec<u8> = Vec::new();
        for _ in 0..max_comp_length {
            compressed.push(0);
        }

        //Compressing and cleaning the compressed value
        Encoder::compress(&mut Encoder::new(), &decompressed, & mut compressed).unwrap();
        let mut b: u8 = 0;
        while b == 0 {
            b = compressed.pop().unwrap();
        }
        compressed.push(b);

        stmt.execute((keys[i], utf16_length, 1, 1, compressed))?;
    }

    len += 10;
    let vacuum_size = fs::metadata(file_path.clone() + "/" + &dir_name + "/ls/data.sqlite").unwrap().len();

    conn.execute(
        "CREATE TABLE if not exists database ( 
        origin TEXT NOT NULL, 
        usage INTEGER NOT NULL DEFAULT 0, 
        last_vacuum_time INTEGER NOT NULL DEFAULT 0, 
        last_analyze_time INTEGER NOT NULL DEFAULT 0, 
        last_vacuum_size INTEGER NOT NULL DEFAULT 0)",
        [])?;

    stmt = conn.prepare("INSERT OR REPLACE INTO database (origin,usage,last_vacuum_time,last_analyze_time,last_vacuum_size) values (?1, ?2, ?3, ?4, ?5)" )?;

    stmt.execute((&website,len,timestamp,0,vacuum_size))?;

    fs::write(file_path.clone() + "/" + &dir_name + "/ls/usage", "");

    Ok(())

}



/**
 * Following function accepts a path to a db file, and a 
 * json string. The json string is parsed as the value and
 * compressed using snappy compression, and is then passed
 * to the db and saved. Note this only applies to Firefox,
 * as firefox is the only browser that I know of that uses
 * this structure. Chromium support in the future...
 */
pub fn write_saved_game (file_path: String, json_string: String, website: String) -> Result<()> {

    let settings: String = serialize_settings(Settings::default());
    write_data(file_path, [json_string,settings], website);

    return Ok(());

}

/**
 * Following function excepts a file location and a world save formatted as a 
 * json string. It then creates a localStorage.setItem() command for the key
 * savedGame, in order for it to be copy pasted into a browser console to 
 * insert the world save
 */
pub fn write_saved_game_command (file: String, json_string: String) -> String {
    let open: String = String::from(r#"localStorage.setItem("savedGame", `"#); //Opening command for localStorage
    let close: String = String::from(r#"`)"#); //Closing command for localStorage

    let output: String = String::from(format!{r"{open}{json_string}{close}"});

    if file != "" {fs::write(file, output.clone()).expect("Error when writing to file")} //Attempting to write localStorage command to file

    return output;

}

/**
 * Following function excepts a file location and settings formatted as a 
 * json string. It then creates a localStorage.setItem() command for the key
 * settings, in order for it to be copy pasted into a browser console to 
 * insert the world save
 */
pub fn write_settings_command (file: String, json_string: String) -> String {
    let open: String = String::from(r#"localStorage.setItem("settings", `"#); //Opening command for localStorage
    let close: String = String::from(r#"`)"#); //Closing command for localStorage

    let output: String = String::from(format!{r"{open}{json_string}{close}"});

    if file != "" {fs::write(file, output.clone()).expect("Error when writing to file")} //Attempting to write localStorage command to file

    return output;

}

/**
 * Following function excepts a file location and an array containing both a 
 * world save and settings formatted as json string. It then creates a 
 * localStorage.setItem() command for the key savedGame and settings, 
 * in order for it to be copy pasted into a browser console to 
 * insert the world save
 */
pub fn write_local_storage_command (file: String, json_strings: [String; 2]) -> String {
    let open: String = String::from(r#"localStorage.setItem("savedGame", `"#); //Opening command for localStorage
    let close: String = String::from(r#"`)"#); //Closing command for localStorage
    let mut string: String = json_strings[0].clone();
    
    let mut output: String = String::from(format!{r"{open}{string}{close}"});
    output += ";";
    
    string = json_strings[1].clone();
    output += &format!{r"{open}{string}{close}"};

    if file != "" {fs::write(file, output.clone()).expect("Error when writing to file")} //Attempting to write localStorage command to file

    return output;

}

/**
 * Following function takes a seed and creates a JSLevel from this seed
 */
pub fn generate_saved_game_from_seed (seed: i64, tile_map: Vec<u8>) -> JSLevel {

    let world_size: i32 = ((tile_map.len()/64) as f64).sqrt() as i32;
    let changed_blocks: HashMap<String, ChangedBlocks> = HashMap::new();
    let level = JSLevel::new(seed, changed_blocks, world_size, 1);

    return deserialize_saved_game(serialize_saved_game(level, tile_map, 2));

}

/**
 * Following function accepts a world size and seed,
 * and then passes them to the js world generation 
 * functionality, and then returns the output as a Vec<>
 */
pub fn get_tile_map (world_size: i32, seed: i64) -> Vec<u8> {
    let y: i32 = 64;
    let level: HashMap<usize, u8> = random_level_worker::start_generation(world_size, seed); //Generating hashmap of all tiles in the world
    let mut tile_map: Vec<u8> = Vec::new();

    for i in 0..world_size * y * world_size {
        tile_map.push(level.get(&(i as usize)).copied().unwrap_or(0)); //Copying hashmap to vec
    }

    return tile_map
}

/**
 * Following function takes a seed and creates a JSLevel from this seed,
 * and then compares it agains the given tilemap to create a json formatted
 * JS world save
 */
#[deprecated(since="0.2.0", note="please use `generate_saved_game_from_seed` instead")]
pub fn serialize_saved_game_from_seed (seed: i64, tile_map: Vec<u8>) -> String {

    let world_size: i32 = ((tile_map.len()/64) as f64).sqrt() as i32;
    let changed_blocks: HashMap<String, ChangedBlocks> = HashMap::new();
    let level = JSLevel::new(seed, changed_blocks, world_size, 1);

    return serialize_saved_game(level, tile_map, 2);
}

/*/**
 * Following function accepts a path to a db file, and a 
 * json string. The json string is parsed as the value and
 * compressed using snappy compression, and is then passed
 * to the db and saved. Note this only applies to Firefox,
 * as firefox is the only browser that I know of that uses
 * this structure. Chromium support in the future...
 */ 
pub fn write_settings (file_path: String, json_string: String, website: String) -> Result<()> {

    let saved_game: String = serialize_saved_game(JSLevel::default(),);
    write_data(file_path, [saved_game,json_string], website);

    return Ok(());

}*/
