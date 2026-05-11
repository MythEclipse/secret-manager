use clap::{Parser, Subcommand};
use secret_manager::infrastructure::db::SqliteSecretRepository;
use secret_manager::application::use_cases::SecretService;
use secret_manager::infrastructure::mcp::run_mcp_server;
use sea_orm::{Database, ConnectionTrait};
use directories::ProjectDirs;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new secret
    Add { name: String, value: String },
    /// Get a secret value
    Get { name: String },
    /// List all secret names
    List,
    /// Start MCP server
    Mcp,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let proj_dirs = ProjectDirs::from("com", "antigravity", "secret-manager")
        .ok_or_else(|| anyhow::anyhow!("could not find config directory"))?;
    let db_path = proj_dirs.data_dir().join("secrets.db");
    let key_path = proj_dirs.data_dir().join(".key");
    std::fs::create_dir_all(proj_dirs.data_dir())?;

    let db_url = format!("sqlite:{}?mode=rwc", db_path.to_string_lossy());
    let db = Database::connect(db_url).await?;

    // Create table if not exists
    db.execute_unprepared("CREATE TABLE IF NOT EXISTS secrets (
        id BLOB PRIMARY KEY,
        name TEXT NOT NULL,
        encrypted_value BLOB NOT NULL,
        nonce BLOB NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    )").await?;

    let repo = SqliteSecretRepository::new(db);

    // Load or generate encryption key
    let encryption_key = if key_path.exists() {
        let key_data = std::fs::read(&key_path)?;
        key_data.try_into().map_err(|_| anyhow::anyhow!("invalid key file"))?
    } else {
        let key: [u8; 32] = rand::random();
        std::fs::write(&key_path, &key)?;
        // Restrict permissions for security
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))?;
        }
        key
    };

    let service = SecretService::new(repo, encryption_key)?;

    match cli.command {
        Commands::Add { name, value } => {
            service.add_secret(name.clone(), &value).await?;
            println!("Secret '{}' added successfully.", name);
        }
        Commands::Get { name } => {
            match service.get_secret(&name).await? {
                Some(val) => println!("{}: {}", name, val),
                None => println!("Secret '{}' not found.", name),
            }
        }
        Commands::List => {
            let secrets = service.list_secrets().await?;
            if secrets.is_empty() {
                println!("No secrets found.");
            } else {
                println!("Secrets:");
                for s in secrets {
                    println!(" - {}", s);
                }
            }
        }
        Commands::Mcp => {
            run_mcp_server(service).await?;
        }
    }

    Ok(())
}
