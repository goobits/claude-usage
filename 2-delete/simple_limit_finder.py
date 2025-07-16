#!/usr/bin/env python3
"""
Simple script to find usage patterns in the specific JSONL files we have
"""
import json
from pathlib import Path

def analyze_actual_files():
    """Analyze the actual JSONL files we found"""
    
    files = [
        "/home/developer/.claude/projects/-workspace/f6bc4859-f162-4d72-9159-a39ef5d9594a.jsonl",
        "/home/developer/.claude/projects/-workspace/b2355a52-4b6d-47f1-a5a9-f6d786aaae6a.jsonl"
    ]
    
    for file_path in files:
        if not Path(file_path).exists():
            print(f"❌ File not found: {file_path}")
            continue
            
        print(f"📁 Analyzing {Path(file_path).name}")
        
        entries_with_usage = []
        interruptions = []
        high_token_entries = []
        
        try:
            with open(file_path, 'r') as f:
                for line_num, line in enumerate(f, 1):
                    try:
                        entry = json.loads(line.strip())
                        
                        # Look for usage data
                        usage = entry.get('message', {}).get('usage', {})
                        if usage:
                            total_tokens = (usage.get('input_tokens', 0) + 
                                          usage.get('output_tokens', 0) + 
                                          usage.get('cache_creation_input_tokens', 0) + 
                                          usage.get('cache_read_input_tokens', 0))
                            
                            entries_with_usage.append({
                                'line': line_num,
                                'timestamp': entry.get('timestamp'),
                                'tokens': total_tokens,
                                'usage': usage,
                                'model': entry.get('message', {}).get('model')
                            })
                            
                            if total_tokens > 100000:  # High usage
                                high_token_entries.append({
                                    'line': line_num,
                                    'tokens': total_tokens,
                                    'model': entry.get('message', {}).get('model')
                                })
                        
                        # Look for interruptions
                        content = entry.get('message', {}).get('content', [])
                        if isinstance(content, list):
                            for item in content:
                                if isinstance(item, dict) and '[Request interrupted by user]' in str(item):
                                    interruptions.append({
                                        'line': line_num,
                                        'timestamp': entry.get('timestamp')
                                    })
                        
                    except json.JSONDecodeError:
                        continue
        
        except Exception as e:
            print(f"❌ Error reading {file_path}: {e}")
            continue
        
        # Report findings
        print(f"  📊 Total entries with usage: {len(entries_with_usage)}")
        print(f"  🔥 High token entries (>100K): {len(high_token_entries)}")
        print(f"  ✂️ User interruptions: {len(interruptions)}")
        
        if high_token_entries:
            print(f"  🎯 Highest token usage:")
            sorted_entries = sorted(high_token_entries, key=lambda x: x['tokens'], reverse=True)
            for entry in sorted_entries[:5]:
                print(f"    Line {entry['line']}: {entry['tokens']:,} tokens ({entry['model']})")
        
        if interruptions:
            print(f"  ✂️ Interruption timestamps:")
            for interrupt in interruptions:
                print(f"    Line {interrupt['line']}: {interrupt['timestamp']}")
        
        # Look for usage patterns that might indicate limits
        if entries_with_usage:
            total_session_tokens = sum(e['tokens'] for e in entries_with_usage)
            print(f"  📈 Total session tokens: {total_session_tokens:,}")
            
            # Group by time windows to see usage patterns
            from datetime import datetime, timedelta
            
            # Sort by timestamp
            valid_entries = []
            for entry in entries_with_usage:
                if entry['timestamp']:
                    try:
                        dt = datetime.fromisoformat(entry['timestamp'].replace('Z', '+00:00'))
                        entry['dt'] = dt
                        valid_entries.append(entry)
                    except:
                        continue
            
            valid_entries.sort(key=lambda x: x['dt'])
            
            if valid_entries:
                start_time = valid_entries[0]['dt']
                end_time = valid_entries[-1]['dt']
                duration = end_time - start_time
                
                print(f"  ⏰ Session duration: {duration}")
                print(f"  🕐 Start: {start_time}")
                print(f"  🏁 End: {end_time}")
                
                # Check if this would fit in 5-hour windows
                if duration > timedelta(hours=5):
                    print(f"  🚨 Session spans >5 hours - would cross Claude windows!")
                else:
                    print(f"  ✅ Session fits within 5-hour window")
        
        print()

if __name__ == "__main__":
    analyze_actual_files()