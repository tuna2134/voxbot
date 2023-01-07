# Voicevox tts bot

## 1. Build binary

```sh
cargo build --release
```

## 2. Create dotenv file.

| name             | description          |
| :---             | :---                 |
| DISCORD_TOKEN    | Discord bot's token  |
| VOICEVOX_API_URL | Voicevox engine urls |

## 3. Running bot

Linux

```sh
./target/release/voxbot
```

Windows

```sh
./target/release/voxbot.exe
```
