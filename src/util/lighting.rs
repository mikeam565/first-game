use bevy::{prelude::*, pbr::CascadeShadowConfigBuilder};

pub fn setup_lighting(mut commands: Commands) {
    commands.spawn( setup_directional_light() );
    commands.insert_resource(AmbientLight {
        color: Color::AZURE,
        brightness: 1.0
    })
}

fn setup_directional_light() -> DirectionalLightBundle {
    // Configure a properly scaled cascade shadow map for this scene (defaults are too large, mesh units are in km)
    let cascade_shadow_config = CascadeShadowConfigBuilder {
        // first_cascade_far_bound: 2.0,
        maximum_distance: 10000.0,
        ..default()
    }.build();
    // Sun
    DirectionalLightBundle {
        directional_light: DirectionalLight {
            color: Color::rgb(0.98, 0.95, 0.82),
            shadows_enabled: true,
            illuminance: 30000.0,
            ..default()
        },
        transform: Transform::from_xyz(0.0,0.0,0.0)
            .looking_at(Vec3::new(-0.2,-1.0,-1.0), Vec3::Y),
        cascade_shadow_config,
        ..default()
    }
}

// fn setup_point_light() -> PointLightBundle {
//     PointLightBundle {
//         point_light: PointLight {
//             intensity: 10000.0,
//             shadows_enabled: true,
//             range: 1000.0,
//             ..default()
//         },
//         transform: Transform::from_xyz(-20.0, 16.0, 34.0),
//         ..default()
//     }
// }

pub struct LightingPlugin;

impl Plugin for LightingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_lighting);
    }
}