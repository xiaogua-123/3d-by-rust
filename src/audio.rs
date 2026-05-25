use bevy::prelude::*;
use bevy::audio::Volume;

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, start_background_music);
    }
}

fn start_background_music(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        AudioPlayer::new(asset_server.load("sounds/music.wav")),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(0.3)),
        Name::new("BackgroundMusic"),
    ));
}
