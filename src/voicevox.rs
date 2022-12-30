use reqwest::{Client, Result};
use serde_json::Value;

pub struct VoiceVox {
    client: Client,
    url: String,
}

impl VoiceVox {
    pub fn new(url: String) -> Self {
        VoiceVox {
            client: Client::new(),
            url,
        }
    }

    pub async fn get_audio_query(&self, text: String, speaker: i32) -> Result<Value> {
        let url = format!("{}/audio_query?speaker={}&text={}", self.url, speaker, text);
        let res = self.client.post(&url).send().await?;
        Ok(res.json().await?)
    }

    pub async fn synthe(&self, speaker: i32, payload: Value) -> Result<bytes::Bytes> {
        let url = format!("{}/synthesis?speaker={}", self.url, speaker);
        let res = self.client.post(&url).json(&payload).send().await?;
        Ok(res.bytes().await?)
    }
}
