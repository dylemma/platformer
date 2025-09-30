use crate::player::PlayerControlParams;
use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use thiserror::Error;

#[derive(Default)]
pub struct PlayerAssetLoader;

#[derive(Debug, Error)]
pub enum PlayerAssetLoaderError {
	#[error("Could not load asset: {0}")]
	Io(#[from] std::io::Error),

	#[error("Could not parse RON: {0}")]
	Ron(#[from] ron::de::SpannedError),
}
impl AssetLoader for PlayerAssetLoader {
	type Asset = PlayerControlParams;
	type Settings = ();
	type Error = PlayerAssetLoaderError;

	async fn load(
		&self,
		reader: &mut dyn Reader,
		_settings: &Self::Settings,
		_load_context: &mut LoadContext<'_>,
	) -> Result<Self::Asset, Self::Error> {
		let mut bytes = Vec::new();
		reader.read_to_end(&mut bytes).await?;
		let player = ron::de::from_bytes::<PlayerControlParams>(&bytes)?;
		Ok(player)
	}

	fn extensions(&self) -> &[&str] {
		&["ron"]
	}
}
