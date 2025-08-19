use claude_usage::parser_wrapper::UnifiedParser;
use std::path::Path;

fn main() {
    let parser = UnifiedParser::new();
    let test_file = Path::new("/workspace/claude-usage/test_data/simple_dedup_test.jsonl");
    
    match parser.parse_jsonl_file(test_file) {
        Ok(entries) => {
            println!("Parsed {} entries:", entries.len());
            for (i, entry) in entries.iter().enumerate() {
                println!("Entry {}: message_id={}, request_id={}, cost_usd={:?}", 
                    i + 1, entry.message.id, entry.request_id, entry.cost_usd);
            }
        }
        Err(e) => {
            println!("Failed to parse: {}", e);
        }
    }
}