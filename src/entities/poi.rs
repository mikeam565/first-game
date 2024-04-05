use bevy::prelude::*;

#[derive(Component)]
pub struct PointOfInterest;

#[derive(Component)]
pub struct ActivePointOfInterest;


fn setup_poi(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = Cuboid::new(5.,5.,5.);
    let material = StandardMaterial {
        base_color: Color::rgb(2.,2.,2.),
        ..default()
    };
    commands.spawn(PbrBundle {
        mesh: meshes.add(mesh),
        material: materials.add(material),
        transform: Transform::from_xyz(-400., 210., 600.),
        ..default()
    })
    .insert(ActivePointOfInterest);
}

pub struct PoiPlugin;

impl Plugin for PoiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_poi);
    }
}