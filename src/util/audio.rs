use std::f32::consts::PI;

use bevy::prelude::*;

use crate::entities::{player::Player, poi::ActivePointOfInterest};

const NUM_STEMS: f32 = 7.;
const AUDIO_DIST: f32 = 600.;

#[derive(Component)]
struct AudioStem {
    threshold: f32
}

fn create_stem_entity(
    file_name: &str,
    threshold: f32,
    asset_server: &Res<AssetServer>
) -> (AudioBundle, AudioStem) {
    let settings = PlaybackSettings {
        mode: bevy::audio::PlaybackMode::Loop,
        ..default()
    };
    (
        AudioBundle {
            source: asset_server.load(file_name.to_owned()),
            settings
        },
        AudioStem { threshold }
    )
}

fn setup_audio(
    mut commands: Commands,
    asset_server: Res<AssetServer>
) {
    let stem_names = ["audio/cello_stem.ogg",
        "audio/choir_stem.ogg",
        "audio/flautando_stems.ogg",
        "audio/fluitvio_stems.ogg",
        "audio/guitarophone_stem.ogg",
        "audio/halfsec_stems.ogg",
        "audio/trumpet_stem.ogg"];

    let _angle_divisions = PI/stem_names.len() as f32;


    for (i, name) in stem_names.iter().enumerate() {
        commands.spawn(create_stem_entity(name, i as f32, &asset_server));
    }
}


fn update_audio(
    _commands: Commands,
    active_poi: Query<&Transform, (With<ActivePointOfInterest>, Without<Player>)>,
    player: Query<&Transform, With<Player>>,
    stems: Query<(&mut AudioSink, &AudioStem)>,
    _time: Res<Time>
) {
    if let Ok(active_trans) = active_poi.get_single() {
        if let Ok(player_trans) = player.get_single() {
            let to_active_poi = (player_trans.translation - active_trans.translation).normalize();
            let angle_btwn = player_trans.forward().angle_between(to_active_poi);
            let dist_btwn = player_trans.translation.distance(active_trans.translation);
            for (sink, audio_stem) in stems.iter() {
                if dist_btwn < AUDIO_DIST/NUM_STEMS * (audio_stem.threshold+1.) {
                    sink.set_volume(angle_btwn/PI);
                } else {
                    sink.set_volume(0.);
                }
            }
        }
    }
}

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_audio);
        app.add_systems(Update, update_audio);
    }
}