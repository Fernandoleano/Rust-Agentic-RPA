use anyhow::Result;
use headless_chrome::{Browser, LaunchOptions, Tab};
use std::path::PathBuf;
use std::sync::Arc;

/// Persistent browser session. Created once, reused for all tasks.
pub struct BrowserSession {
    _browser: Browser,
    pub tab: Arc<Tab>,
}

impl BrowserSession {
    pub fn launch() -> Result<Self> {
        // 1. Try to connect to existing Chrome (Attach Mode)
        eprintln!("[Hands] 🔗 Attempting to attach to existing Chrome on port 9222...");
        if let Ok(browser) = Browser::connect("http://127.0.0.1:9222".to_string()) {
            eprintln!("[Hands] ✅ Attached to existing Chrome!");

            // get_tabs() returns Arc<Mutex<Vec<Arc<Tab>>>>
            let tab = {
                let tabs_lock = browser.get_tabs();
                let tabs = tabs_lock.lock().unwrap();
                if let Some(t) = tabs.first() {
                    eprintln!("[Hands] Using existing tab.");
                    t.clone()
                } else {
                    eprintln!("[Hands] No tabs found, creating new one.");
                    browser.new_tab()?
                }
            };

            return Ok(Self {
                _browser: browser,
                tab,
            });
        }

        eprintln!("[Hands] ⚠️  Could not attach. Launching Shadow Profile...");

        let chrome_path = find_chrome()?;

        // Use a shadow profile to avoid locking the real one.
        // If it already exists, use it as is (so agent logins persist).
        let agent_profile = std::env::current_dir()?.join("agent_profile");

        if !agent_profile.exists() {
            eprintln!(
                "[Hands] Creating new shadow profile at: {:?}",
                agent_profile
            );
            std::fs::create_dir_all(&agent_profile)?;

            // Initial Sync
            kill_chrome_processes();
            std::thread::sleep(std::time::Duration::from_secs(2));

            if let Err(e) = sync_profile(&agent_profile) {
                eprintln!("[Hands] Warning: Profile sync failed: {}", e);
            }
        } else {
            eprintln!("[Hands] Using existing persistent shadow profile.");
        }

        let options = LaunchOptions {
            headless: false,
            path: Some(chrome_path),
            user_data_dir: Some(agent_profile.clone()),
            // port: Some(9222), // Let headless_chrome pick a random port to avoid conflicts
            args: vec![
                std::ffi::OsStr::new("--no-first-run"),
                std::ffi::OsStr::new("--no-default-browser-check"),
                // Anti-bot flags
                std::ffi::OsStr::new("--disable-blink-features=AutomationControlled"),
                std::ffi::OsStr::new("--disable-infobars"),
                std::ffi::OsStr::new("--restore-last-session"),
                std::ffi::OsStr::new("--password-store=basic"),
            ],
            idle_browser_timeout: std::time::Duration::from_secs(60),
            ..Default::default()
        };

        eprintln!("[Hands] Starting Chrome (Shadow Profile)...");
        let browser = Browser::new(options).map_err(|e| {
            eprintln!("[Hands] Browser launch failed: {}", e);
            anyhow::anyhow!("Browser launch failed: {}", e)
        })?;

        eprintln!("[Hands] Chrome started, creating tab...");
        let tab = browser.new_tab()?;
        tab.navigate_to("about:blank")?;

        eprintln!("[Hands] Chrome ready.");

        Ok(Self {
            _browser: browser,
            tab,
        })
    }
    pub fn new_tab(&mut self) -> Result<()> {
        let tab = self._browser.new_tab()?;
        self.tab = tab;
        Ok(())
    }
}

fn sync_profile(agent_profile: &std::path::Path) -> Result<()> {
    let local_data = dirs::data_local_dir().ok_or_else(|| anyhow::anyhow!("No AppData/Local"))?;
    let real_user_data = local_data.join("Google").join("Chrome").join("User Data");

    if !real_user_data.exists() {
        return Ok(());
    }

    // 1. Copy Local State (Key for decrypting cookies)
    let _ = std::fs::copy(
        real_user_data.join("Local State"),
        agent_profile.join("Local State"),
    );

    // 2. Copy Default folder contents
    let real_default = real_user_data.join("Default");
    let agent_default = agent_profile.join("Default");
    std::fs::create_dir_all(&agent_default)?;

    let files = [
        "Cookies",
        "Cookies-journal",
        "Login Data",
        "Login Data-journal",
        "Web Data",
        "Web Data-journal",
        "History",
        "History-journal",
        "Favicons",
        "Bookmarks",
        "Preferences",
        "Secure Preferences",
        "Affiliation Database",
        "Affiliation Database-journal",
    ];

    for name in &files {
        let src = real_default.join(name);
        if src.exists() {
            let _ = std::fs::copy(&src, agent_default.join(name));
        }
    }

    // 3. Copy Network folder (New Chrome Cookies location)
    let real_network = real_default.join("Network");
    let agent_network = agent_default.join("Network");
    if real_network.exists() {
        let _ = std::fs::create_dir_all(&agent_network);
        let network_files = [
            "Cookies",
            "Cookies-journal",
            "Trust Tokens",
            "Network Persistent State",
        ];
        for name in &network_files {
            let src = real_network.join(name);
            if src.exists() {
                let _ = std::fs::copy(&src, agent_network.join(name));
            }
        }
    }

    Ok(())
}

// Helper to find Chrome executable
fn find_chrome() -> Result<PathBuf> {
    let candidates = [
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        &format!(
            r"C:\Users\{}\AppData\Local\Google\Chrome\Application\chrome.exe",
            std::env::var("USERNAME").unwrap_or("Default".to_string())
        ),
    ];

    for path in &candidates {
        let p = PathBuf::from(path);
        if p.exists() {
            return Ok(p);
        }
    }

    anyhow::bail!("Chrome executable not found. Please ensure Google Chrome is installed.")
}

fn kill_chrome_processes() {
    let _ = std::process::Command::new("taskkill")
        .args(["/F", "/IM", "chrome.exe"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}
