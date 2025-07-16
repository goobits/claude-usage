#!/usr/bin/env python3
"""
Debug script to find usage limit hit messages in real Claude JSONL files
"""
import json
import glob
from pathlib import Path
from datetime import datetime

def analyze_limit_patterns():
    """Find usage limit patterns in real JSONL files"""
    
    # Find actual JSONL files
    jsonl_files = list(Path.home().glob(".claude/projects/*/conversation_*.jsonl"))
    
    print(f"🔍 Found {len(jsonl_files)} JSONL files")
    print()
    
    limit_indicators = []
    high_usage_sessions = []
    conversation_cuts = []
    model_changes = []
    
    for jsonl_file in jsonl_files:
        print(f"📁 Analyzing {jsonl_file.name}...")
        
        try:
            with open(jsonl_file, 'r') as f:
                entries = []
                for line_num, line in enumerate(f, 1):
                    try:
                        entry = json.loads(line.strip())
                        entries.append((line_num, entry))
                    except json.JSONDecodeError:
                        continue
                
                # Analyze this conversation for patterns
                analyze_conversation(jsonl_file, entries, limit_indicators, 
                                   high_usage_sessions, conversation_cuts, model_changes)
                
        except Exception as e:
            print(f"❌ Error reading {jsonl_file}: {e}")
    
    # Report findings
    print(f"\n🚨 LIMIT INDICATORS FOUND:")
    for indicator in limit_indicators:
        print(f"📁 {indicator['file']}:{indicator['line']}")
        print(f"🔑 Type: {indicator['type']}")
        print(f"📝 Content: {indicator['content'][:100]}...")
        print(f"💎 Tokens: {indicator.get('tokens', 'N/A')}")
        print("-" * 80)
    
    print(f"\n🔥 HIGH USAGE SESSIONS:")
    for session in high_usage_sessions:
        print(f"📁 {session['file']}")
        print(f"🎯 Total tokens: {session['total_tokens']:,}")
        print(f"📊 Message count: {session['message_count']}")
        print(f"⏰ Duration: {session['duration']}")
        print("-" * 80)
    
    print(f"\n✂️ CONVERSATION CUTS:")
    for cut in conversation_cuts:
        print(f"📁 {cut['file']}:{cut['line']}")
        print(f"📝 Last message: {cut['last_content'][:100]}...")
        print(f"🎯 Tokens at cut: {cut['tokens']}")
        print("-" * 80)
    
    print(f"\n🔄 MODEL CHANGES:")
    for change in model_changes:
        print(f"📁 {change['file']}:{change['line']}")
        print(f"🔄 From: {change['from_model']} → To: {change['to_model']}")
        print(f"🎯 Tokens when changed: {change['tokens']}")
        print("-" * 80)

def analyze_conversation(jsonl_file, entries, limit_indicators, high_usage_sessions, 
                        conversation_cuts, model_changes):
    """Analyze a single conversation for limit patterns"""
    
    total_tokens = 0
    message_count = 0
    start_time = None
    end_time = None
    last_model = None
    last_message_content = ""
    
    for line_num, entry in entries:
        # Skip summaries
        if entry.get('type') == 'summary':
            continue
            
        # Extract timestamp
        timestamp = entry.get('timestamp')
        if timestamp:
            try:
                dt = datetime.fromisoformat(timestamp.replace('Z', '+00:00'))
                if start_time is None:
                    start_time = dt
                end_time = dt
            except:
                pass
        
        # Check for limit-related keywords in content
        entry_str = json.dumps(entry).lower()
        limit_keywords = [
            'limit reached', 'usage limit', 'rate limit', 'exceeded',
            'max usage', 'quota', 'throttle', 'blocked', 'restricted',
            'too many', 'slow down', 'capacity', 'unavailable'
        ]
        
        for keyword in limit_keywords:
            if keyword in entry_str:
                limit_indicators.append({
                    'file': jsonl_file.name,
                    'line': line_num,
                    'type': 'keyword',
                    'keyword': keyword,
                    'content': entry_str[:200],
                    'tokens': entry.get('message', {}).get('usage', {})
                })
        
        # Check for user interruptions
        if (entry.get('type') == 'user' and 
            '[Request interrupted by user]' in str(entry.get('message', {}).get('content', []))):
            conversation_cuts.append({
                'file': jsonl_file.name,
                'line': line_num,
                'last_content': last_message_content,
                'tokens': total_tokens
            })
        
        # Track model changes
        current_model = entry.get('message', {}).get('model')
        if current_model and last_model and current_model != last_model:
            model_changes.append({
                'file': jsonl_file.name,
                'line': line_num,
                'from_model': last_model,
                'to_model': current_model,
                'tokens': total_tokens
            })
        if current_model:
            last_model = current_model
        
        # Accumulate usage data
        usage = entry.get('message', {}).get('usage', {})
        if usage:
            message_count += 1
            total_tokens += (usage.get('input_tokens', 0) + 
                           usage.get('output_tokens', 0) + 
                           usage.get('cache_creation_input_tokens', 0) + 
                           usage.get('cache_read_input_tokens', 0))
        
        # Track last message content
        content = entry.get('message', {}).get('content', [])
        if content and isinstance(content, list):
            for item in content:
                if isinstance(item, dict) and item.get('type') == 'text':
                    last_message_content = item.get('text', '')[:200]
    
    # Check if this is a high usage session
    if total_tokens > 500000:  # Half a million tokens
        duration = "Unknown"
        if start_time and end_time:
            duration = str(end_time - start_time)
        
        high_usage_sessions.append({
            'file': jsonl_file.name,
            'total_tokens': total_tokens,
            'message_count': message_count,
            'duration': duration
        })

if __name__ == "__main__":
    analyze_limit_patterns()