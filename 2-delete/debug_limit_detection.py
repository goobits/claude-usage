#!/usr/bin/env python3
"""
Debug script to find usage limit hit messages in Claude JSONL files
"""
import json
import glob
import re
from pathlib import Path
from datetime import datetime

def find_limit_messages():
    """Find all potential usage limit messages in JSONL files"""
    
    # Keywords that might indicate usage limits
    limit_keywords = [
        'usage limit', 'rate limit', 'max usage', 'limit reached', 'exceeded',
        'quota', 'throttle', 'blocked', 'restricted', 'unavailable',
        'too many', 'slow down', 'wait', 'retry', 'capacity',
        'error', 'failed', 'denied', 'forbidden'
    ]
    
    # Find all JSONL files
    home_dir = Path.home()
    claude_paths = [
        home_dir / ".claude" / "projects",
        home_dir / ".claude" / "vms"
    ]
    
    jsonl_files = []
    for claude_path in claude_paths:
        if claude_path.exists():
            jsonl_files.extend(claude_path.glob("**/conversation_*.jsonl"))
    
    print(f"🔍 Scanning {len(jsonl_files)} JSONL files for limit messages...")
    print(f"📂 Looking in: {[str(p) for p in claude_paths]}")
    print()
    
    limit_messages = []
    incomplete_messages = []
    high_token_entries = []
    
    total_entries = 0
    
    for jsonl_file in jsonl_files:
        try:
            with open(jsonl_file, 'r') as f:
                for line_num, line in enumerate(f, 1):
                    try:
                        entry = json.loads(line.strip())
                        total_entries += 1
                        
                        # Convert entry to string for keyword searching
                        entry_str = json.dumps(entry).lower()
                        
                        # Check for limit keywords
                        for keyword in limit_keywords:
                            if keyword in entry_str:
                                limit_messages.append({
                                    'file': str(jsonl_file),
                                    'line': line_num,
                                    'keyword': keyword,
                                    'entry': entry,
                                    'timestamp': entry.get('timestamp'),
                                    'message_id': entry.get('messageId'),
                                    'tokens': entry.get('message', {}).get('usage', {})
                                })
                                break
                        
                        # Check for incomplete/cut-off messages
                        message = entry.get('message', {})
                        content = message.get('content', [])
                        if content and isinstance(content, list):
                            for item in content:
                                if isinstance(item, dict) and item.get('type') == 'text':
                                    text = item.get('text', '')
                                    # Look for incomplete sentences or abrupt endings
                                    if (text and len(text) > 100 and 
                                        not text.rstrip().endswith(('.', '!', '?', '`', '"', "'"))):
                                        incomplete_messages.append({
                                            'file': str(jsonl_file),
                                            'line': line_num,
                                            'entry': entry,
                                            'text_end': text[-100:],
                                            'tokens': entry.get('message', {}).get('usage', {})
                                        })
                        
                        # Check for high token usage (potential limits)
                        usage = message.get('usage', {})
                        if usage:
                            total_tokens = (usage.get('input_tokens', 0) + 
                                          usage.get('output_tokens', 0) + 
                                          usage.get('cache_creation_input_tokens', 0) + 
                                          usage.get('cache_read_input_tokens', 0))
                            
                            # Flag entries with unusually high token usage
                            if total_tokens > 50000:  # Arbitrary threshold
                                high_token_entries.append({
                                    'file': str(jsonl_file),
                                    'line': line_num,
                                    'entry': entry,
                                    'total_tokens': total_tokens,
                                    'usage': usage
                                })
                    
                    except json.JSONDecodeError:
                        continue
                        
        except Exception as e:
            print(f"❌ Error reading {jsonl_file}: {e}")
    
    print(f"📊 Analyzed {total_entries} total entries")
    print()
    
    # Report findings
    print("🚨 LIMIT-RELATED MESSAGES:")
    if limit_messages:
        for msg in limit_messages[:10]:  # Show first 10
            print(f"📁 {Path(msg['file']).name}:{msg['line']}")
            print(f"🔑 Keyword: {msg['keyword']}")
            print(f"⏰ Timestamp: {msg['timestamp']}")
            print(f"💬 Message ID: {msg['message_id']}")
            print(f"🎯 Tokens: {msg['tokens']}")
            print(f"📝 Entry snippet:")
            print(json.dumps(msg['entry'], indent=2)[:500] + "...")
            print("-" * 80)
    else:
        print("✅ No explicit limit messages found")
    
    print()
    print("✂️  INCOMPLETE MESSAGES (potential cutoffs):")
    if incomplete_messages:
        for msg in incomplete_messages[:5]:  # Show first 5
            print(f"📁 {Path(msg['file']).name}:{msg['line']}")
            print(f"🎯 Tokens: {msg['tokens']}")
            print(f"📝 Text ending: ...{msg['text_end']}")
            print("-" * 80)
    else:
        print("✅ No incomplete messages found")
    
    print()
    print("🔥 HIGH TOKEN USAGE:")
    if high_token_entries:
        # Sort by token count
        high_token_entries.sort(key=lambda x: x['total_tokens'], reverse=True)
        for msg in high_token_entries[:5]:  # Show top 5
            print(f"📁 {Path(msg['file']).name}:{msg['line']}")
            print(f"🎯 Total tokens: {msg['total_tokens']:,}")
            print(f"📊 Usage breakdown: {msg['usage']}")
            print("-" * 80)
    else:
        print("✅ No high token usage found")
    
    # Summary statistics
    print()
    print("📈 SUMMARY:")
    print(f"Total entries analyzed: {total_entries:,}")
    print(f"Limit-related messages: {len(limit_messages)}")
    print(f"Incomplete messages: {len(incomplete_messages)}")
    print(f"High token entries: {len(high_token_entries)}")
    
    return {
        'limit_messages': limit_messages,
        'incomplete_messages': incomplete_messages,
        'high_token_entries': high_token_entries,
        'total_entries': total_entries
    }

if __name__ == "__main__":
    results = find_limit_messages()
    
    # Save results for further analysis
    with open('/workspace/limit_detection_results.json', 'w') as f:
        json.dump(results, f, indent=2, default=str)
    
    print(f"\n💾 Full results saved to: /workspace/limit_detection_results.json")