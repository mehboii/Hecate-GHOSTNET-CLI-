use colored::Colorize;

/// Inner width of the banner box (chars between the side borders).
const W: usize = 47;

// Brand gradient sampled from the GhostNet site: indigo/violet → teal/cyan.
const GRAD_START: (u8, u8, u8) = (124, 108, 240); // #7C6CF0
const GRAD_END: (u8, u8, u8) = (45, 212, 191); //   #2DD4BF

/// The N11X Collective branding banner, printed at the top of CLI output.
pub fn print_banner() {
    println!();
    println!("{}", format!("╔{}╗", "═".repeat(W)).white().bold());
    frame_plain("");
    frame_gradient("N 1 1 X   C O L L E C T I V E");
    frame_gradient("· H E C A T E ·");
    frame_plain("");
    frame_plain("encrypted · decentralized · private");
    frame_plain("");
    println!("{}", format!("╚{}╝", "═".repeat(W)).white().bold());
    println!();
}

/// Greets the current user. All white per brand guidelines.
pub fn welcome() {
    let user = current_user();
    println!(
        "{} {}{}",
        "Welcome,".white().bold(),
        user.white().bold(),
        "!".white().bold()
    );
    println!(
        "{}",
        "   You're plugged into the GhostNet mesh — N11X Collective.".white()
    );
    println!();
}

/// Apply the brand gradient to arbitrary text (used for the shell prompt).
pub fn brand(text: &str) -> String {
    gradient(text)
}

fn current_user() -> String {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "ghost".to_string())
}

/// White, centered line inside the box.
fn frame_plain(text: &str) {
    let border = "║".white().bold();
    println!("{}{}{}", border, center_white(text), border);
}

/// Gradient-branded, centered line inside the box.
fn frame_gradient(text: &str) {
    let visible = text.chars().count();
    let pad = W.saturating_sub(visible);
    let left = pad / 2;
    let right = pad - left;
    let border = "║".white().bold();
    println!(
        "{}{}{}{}{}",
        border,
        " ".repeat(left),
        gradient(text),
        " ".repeat(right),
        border
    );
}

fn center_white(text: &str) -> String {
    let visible = text.chars().count().min(W);
    let pad = W - visible;
    let left = pad / 2;
    let right = pad - left;
    format!(
        "{}{}{}",
        " ".repeat(left),
        text.white().bold(),
        " ".repeat(right)
    )
}

/// Colour each character along the indigo→cyan brand gradient.
fn gradient(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut out = String::new();
    for (i, ch) in chars.iter().enumerate() {
        let t = if n <= 1 { 0.0 } else { i as f32 / (n - 1) as f32 };
        let r = lerp(GRAD_START.0, GRAD_END.0, t);
        let g = lerp(GRAD_START.1, GRAD_END.1, t);
        let b = lerp(GRAD_START.2, GRAD_END.2, t);
        out.push_str(&ch.to_string().truecolor(r, g, b).bold().to_string());
    }
    out
}

fn lerp(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}
