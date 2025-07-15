use regex::Regex;

fn main() {
    let template = std::fs::read_to_string("templates/cpm-spear.md").unwrap();
    
    // Try different regex patterns
    let patterns = vec![
        r"(?s)\[\[LLM_PROMPT:\s*([^\]]+?)\]\]",
        r"\[\[LLM_PROMPT:\s*([^\]]+)\]\]",
        r"(?s)\[\[LLM_PROMPT:\s*([^]]+?)\]\]",
    ];
    
    for (i, pattern) in patterns.iter().enumerate() {
        println!("Pattern {}: {}", i + 1, pattern);
        let re = Regex::new(pattern).unwrap();
        let matches: Vec<_> = re.captures_iter(&template).collect();
        println!("Found {} matches", matches.len());
        
        for (j, cap) in matches.iter().enumerate() {
            let full_match = cap.get(0).unwrap().as_str();
            let prompt_text = cap.get(1).unwrap().as_str();
            println!("  Match {}: {} chars", j + 1, full_match.len());
            println!("    Prompt: {}", prompt_text.replace('\n', ' ').chars().take(60).collect::<String>());
        }
        println!();
    }
}
