use std::collections::BTreeMap;

use anyhow::Result;

use log::{debug, error};

use memflow::prelude::v1::*;

use pelite::pattern;
use pelite::pattern::{Atom, save_len};
use pelite::pe64::{Pe, PeView, Rva};

use phf::{Map, phf_map};

pub type OffsetMap = BTreeMap<String, BTreeMap<String, Rva>>;

macro_rules! pattern_map {
    ($($module:ident => {
        $($name:expr => $pattern:expr $(=> $callback:expr)?),+ $(,)?
    }),+ $(,)?) => {
        $(
            mod $module {
                use super::*;

                pub(super) const PATTERNS: Map<
                    &'static str,
                    (
                        &'static str, // Pattern string
                        &'static [Atom], // Compiled pattern
                        Option<fn(&PeView, &mut BTreeMap<String, Rva>, Rva)>,
                    ),
                > = phf_map! {
                    $($name => ($pattern.into(), pattern!($pattern), $($callback)?)),+
                };

                pub fn offsets(view: PeView<'_>) -> (BTreeMap<String, Rva>, BTreeMap<String, String>) {
                    let mut offset_map = BTreeMap::new();
                    let mut pattern_map = BTreeMap::new();

                    for (&name, (pattern_str, compiled_pat, callback)) in &PATTERNS {
                        pattern_map.insert(name.to_string(), pattern_str.to_string());

                        let mut save = vec![0; save_len(compiled_pat)];

                        if !view.scanner().finds_code(compiled_pat, &mut save) {
                            error!("outdated pattern: {}", name);

                            continue;
                        }

                        let rva = save[1];

                        offset_map.insert(name.to_string(), rva);

                        if let Some(callback) = callback {
                            callback(&view, &mut offset_map, rva);
                        }
                    }

                    for (name, value) in &offset_map {
                        debug!(
                            "found offset: {} at {:#X} ({}.dll + {:#X})",
                            name,
                            *value as u64 + view.optional_header().ImageBase,
                            stringify!($module),
                            value
                        );
                    }

                    (offset_map, pattern_map)
                }
            }
        )+
    };
}

pattern_map! {
    client => {
        "dwCSGOInput" => "488905${'} 0f57c0 0f1105" => Some(|view, map, rva| {
            let mut save = [0; 2];

            if view.scanner().finds_code(pattern!("f2410f108430u4"), &mut save) {
                map.insert("dwViewAngles".to_string(), rva + save[1]);
            }
        }),
        "dwEntityList" => "488935${'} 4885f6" => None,
        "dwGameEntitySystem" => "488b1d${'} 48891d" => None,
        "dwGameEntitySystem_highestEntityIndex" => "8b81u2?? 8902 488bc2 c3 cccccccc 48895c24? 48896c24" => None,
        "dwGameRules" => "48891d${'} ff15${} 84c0" => None,
        "dwGlobalVars" => "488915${'} 488942" => None,
        "dwGlowManager" => "488b05${'} c3 cccccccccccccccc 8b41" => None,
        "dwLocalPlayerController" => "488905${'} 8b9e" => None,
        "dwPlantedC4" => "488b15${'} 41ffc0" => None,
        "dwPrediction" => "488d05${'} c3 cccccccccccccccc 4883ec? 8b0d" => Some(|_view, map, rva| {
            map.insert("dwLocalPlayerPawn".to_string(), rva + 0x180);
        }),
        "dwSensitivity" => "488d0d${[8]'} 440f28c1 0f28f3 0f28fa e8" => None,
        "dwSensitivity_sensitivity" => "ff50u1 4c8bc6 488d55? 488bcf e8${} 84c0 0f85${} 4c8d45? 8bd3 488bcf e8${} e9${} f30f1006" => None,
        "dwViewMatrix" => "488d0d${'} 48c1e006" => None,
        "dwViewRender" => "488905${'} 488bc8 4885c0" => None,
        "dwWeaponC4" => "488b15${'} 488b5c24? ffc0 8905[4] 488bc7" => None,
    },
    engine2 => {
        "dwBuildNumber" => "8905${'} 488d0d${} ff15${} 488b0d" => None,
        "dwNetworkGameClient" => "48893d${'} 488d15" => None,
        "dwNetworkGameClient_clientTickCount" => "8b81u4 c3 cccccccccccccccccc 8b81${} c3 cccccccccccccccccc 83b9" => None,
        "dwNetworkGameClient_deltaTick" => "89b3u4 8b45" => None,
        "dwNetworkGameClient_isBackgroundMap" => "0fb681u4 c3 cccccccccccccccc 0fb681${} c3 cccccccccccccccc 48895c24" => None,
        "dwNetworkGameClient_localPlayer" => "4883c0u1 488d0440 8b0cc1" => Some(|_view, map, rva| {
            // .text 48 83 C0 0A | add rax, 0Ah
            // .text 48 8D 04 40 | lea rax, [rax + rax * 2]
            // .text 8B 0C C1    | mov ecx, [rcx + rax * 8]
            map.insert("dwNetworkGameClient_localPlayer".to_string(), (rva + (rva * 2)) * 8);
        }),
        "dwNetworkGameClient_maxClients" => "8b81u4 c3cccccccccccccccccc 8b81${} ffc0" => None,
        "dwNetworkGameClient_serverTickCount" => "8b81u4 c3 cccccccccccccccccc 83b9" => None,
        "dwNetworkGameClient_signOnState" => "448b81u4 488d0d" => None,
        "dwWindowHeight" => "8b05${'} 8903" => None,
        "dwWindowWidth" => "8b05${'} 8907" => None,
    },
    input_system => {
        "dwInputSystem" => "488905${'} 488d05" => None,
    },
    matchmaking => {
        "dwGameTypes" => "488d0d${'} 33d2" => None,
        "dwGameTypes_mapName" => "488b81u4 4885c074? 4883c0" => None,
    },
    soundsystem => {
        "dwSoundSystem" => "488d05${'} c3 cccccccccccccccc 488915" => None,
        "dwSoundSystem_engineViewData" => "0f1147u1 0f104b" => None,
    },
}

pub type PatternMap = BTreeMap<String, BTreeMap<String, String>>;

pub fn offsets<P: Process + MemoryView>(process: &mut P) -> Result<(OffsetMap, PatternMap)> {
    let mut offset_map_all = BTreeMap::new();
    let mut pattern_map_all = BTreeMap::new();

    let modules: [(&str, fn(PeView) -> (BTreeMap<String, u32>, BTreeMap<String, String>)); 5] = [
        ("client.dll", client::offsets),
        ("engine2.dll", engine2::offsets),
        ("inputsystem.dll", input_system::offsets),
        ("matchmaking.dll", matchmaking::offsets),
        ("soundsystem.dll", soundsystem::offsets),
    ];

    for (module_name, offsets_fn) in &modules {
        let module = process.module_by_name(module_name)?;

        let buf = process
            .read_raw(module.base, module.size as _)
            .data_part()?;

        let view = PeView::from_bytes(&buf)?;

        let (module_offsets, module_patterns) = offsets_fn(view);
        offset_map_all.insert(module_name.to_string(), module_offsets);
        pattern_map_all.insert(module_name.to_string(), module_patterns);
    }

    Ok((offset_map_all, pattern_map_all))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::Value;

    use super::*;

    fn setup() -> Result<IntoProcessInstanceArcBox<'static>> {
        let os = memflow_native::create_os(&OsArgs::default(), LibArc::default())?;

        let process = os.into_process_by_name("cs2.exe")?;

        Ok(process)
    }

    fn get_class_field_value(module_name: &str, class_name: &str, field_name: &str) -> Option<u64> {
        let content =
            fs::read_to_string(format!("output/{}.json", module_name.replace(".", "_"))).ok()?;

        let value: Value = serde_json::from_str(&content).ok()?;

        value
            .get(module_name)?
            .get("classes")?
            .get(class_name)?
            .get("fields")?
            .get(field_name)?
            .as_u64()
    }

    fn get_offset_value(module_name: &str, offset_name: &str) -> Option<u64> {
        let content = fs::read_to_string("output/offsets.json").ok()?;
        let value: Value = serde_json::from_str(&content).ok()?;

        let offset = value.get(module_name)?.get(offset_name)?;

        offset.as_u64()
    }

    #[test]
    fn build_number() -> Result<()> {
        let mut process = setup()?;

        let engine_base = process.module_by_name("engine2.dll")?.base;

        let offset = get_offset_value("engine2.dll", "dwBuildNumber").unwrap();

        let build_number: u32 = process.read(engine_base + offset).data_part()?;

        println!("build number: {}", build_number);

        Ok(())
    }

    #[test]
    fn global_vars() -> Result<()> {
        let mut process = setup()?;

        let client_base = process.module_by_name("client.dll")?.base;

        let offset = get_offset_value("client.dll", "dwGlobalVars").unwrap();

        let global_vars: u64 = process.read(client_base + offset).data_part()?;

        let cur_map_name = {
            let addr = process
                .read_addr64((global_vars + 0x180).into())
                .data_part()?;

            process.read_utf8(addr, 128).data_part()?
        };

        println!("current map name: {}", cur_map_name);

        Ok(())
    }

    #[test]
    fn local_player_controller() -> Result<()> {
        let mut process = setup()?;

        let client_base = process.module_by_name("client.dll")?.base;

        let local_player_controller_offset =
            get_offset_value("client.dll", "dwLocalPlayerController").unwrap();

        let player_name_offset =
            get_class_field_value("client.dll", "CBasePlayerController", "m_iszPlayerName")
                .unwrap();

        let local_player_controller: u64 = process
            .read(client_base + local_player_controller_offset)
            .data_part()?;

        let player_name = process
            .read_utf8((local_player_controller + player_name_offset).into(), 4096)
            .data_part()?;

        println!("local player name: {}", player_name);

        Ok(())
    }

    #[test]
    fn local_player_pawn() -> Result<()> {
        #[derive(Debug, Pod)]
        #[repr(C)]
        struct Vector3D {
            x: f32,
            y: f32,
            z: f32,
        }

        let mut process = setup()?;

        let client_base = process.module_by_name("client.dll")?.base;

        let local_player_pawn_offset = get_offset_value("client.dll", "dwLocalPlayerPawn").unwrap();

        let game_scene_node_offset =
            get_class_field_value("client.dll", "C_BaseEntity", "m_pGameSceneNode").unwrap();

        let vec_abs_origin_offset =
            get_class_field_value("client.dll", "CGameSceneNode", "m_vecAbsOrigin").unwrap();

        let local_player_pawn: u64 = process
            .read(client_base + local_player_pawn_offset)
            .data_part()?;

        let game_scene_node: u64 = process
            .read((local_player_pawn + game_scene_node_offset).into())
            .data_part()?;

        let vec_abs_origin: Vector3D = process
            .read((game_scene_node + vec_abs_origin_offset).into())
            .data_part()?;

        println!("local player origin: {:?}", vec_abs_origin);

        Ok(())
    }

    #[test]
    fn window_size() -> Result<()> {
        let mut process = setup()?;

        let engine_base = process.module_by_name("engine2.dll")?.base;

        let window_width_offset = get_offset_value("engine2.dll", "dwWindowWidth").unwrap();
        let window_height_offset = get_offset_value("engine2.dll", "dwWindowHeight").unwrap();

        let window_width: u32 = process
            .read(engine_base + window_width_offset)
            .data_part()?;

        let window_height: u32 = process
            .read(engine_base + window_height_offset)
            .data_part()?;

        println!("window size: {}x{}", window_width, window_height);

        Ok(())
    }
}
