#!/usr/bin/env python3
"""
Create sliding window detector for Claude usage limits based on actual patterns
"""
import json
from pathlib import Path
from datetime import datetime, timedelta
from collections import defaultdict

def create_sliding_window_detector():
    """Create a detector that finds 5-hour windows and identifies plan limits"""
    
    # Find JSONL files
    jsonl_files = list(Path.home().glob(".claude/projects/*/conversation_*.jsonl"))
    
    print(f"🔍 Creating sliding window detector from {len(jsonl_files)} files")
    
    # Parse all entries with timestamps and usage
    all_entries = []
    
    for jsonl_file in jsonl_files:
        try:
            with open(jsonl_file, 'r') as f:
                for line_num, line in enumerate(f, 1):
                    try:
                        entry = json.loads(line.strip())
                        
                        # Skip summaries and entries without usage
                        if entry.get('type') == 'summary':
                            continue
                            
                        timestamp = entry.get('timestamp')
                        usage = entry.get('message', {}).get('usage', {})
                        
                        if timestamp and usage:
                            # Parse timestamp
                            try:
                                dt = datetime.fromisoformat(timestamp.replace('Z', '+00:00'))
                                
                                total_tokens = (usage.get('input_tokens', 0) + 
                                              usage.get('output_tokens', 0) + 
                                              usage.get('cache_creation_input_tokens', 0) + 
                                              usage.get('cache_read_input_tokens', 0))
                                
                                if total_tokens > 0:
                                    all_entries.append({
                                        'timestamp': dt,
                                        'tokens': total_tokens,
                                        'file': jsonl_file.name,
                                        'line': line_num,
                                        'entry': entry
                                    })
                            except:
                                continue
                                
                    except json.JSONDecodeError:
                        continue
        except Exception as e:
            print(f"❌ Error reading {jsonl_file}: {e}")
    
    # Sort by timestamp
    all_entries.sort(key=lambda x: x['timestamp'])
    
    print(f"📊 Found {len(all_entries)} entries with usage data")
    
    # Detect 5-hour usage windows
    windows = detect_5_hour_windows(all_entries)
    
    # Analyze patterns
    analyze_window_patterns(windows)
    
    return windows

def detect_5_hour_windows(entries):
    """Detect 5-hour usage windows based on gaps in activity"""
    
    windows = []
    current_window = []
    
    for i, entry in enumerate(entries):
        if i == 0:
            # First entry starts first window
            current_window = [entry]
            window_start = entry['timestamp']
        else:
            prev_entry = entries[i-1]
            gap = entry['timestamp'] - prev_entry['timestamp']
            
            # If gap > 5 hours, start new window
            if gap > timedelta(hours=5):
                # Close current window
                if current_window:
                    windows.append({
                        'start': current_window[0]['timestamp'],
                        'end': current_window[-1]['timestamp'],
                        'entries': current_window,
                        'total_tokens': sum(e['tokens'] for e in current_window),
                        'duration': current_window[-1]['timestamp'] - current_window[0]['timestamp']
                    })
                
                # Start new window
                current_window = [entry]
                window_start = entry['timestamp']
            else:
                # Add to current window
                current_window.append(entry)
    
    # Don't forget the last window
    if current_window:
        windows.append({
            'start': current_window[0]['timestamp'],
            'end': current_window[-1]['timestamp'],
            'entries': current_window,
            'total_tokens': sum(e['tokens'] for e in current_window),
            'duration': current_window[-1]['timestamp'] - current_window[0]['timestamp']
        })
    
    return windows

def analyze_window_patterns(windows):
    """Analyze the detected windows for limit patterns"""
    
    print(f"\n🕐 DETECTED 5-HOUR WINDOWS ({len(windows)} total):")
    print("=" * 80)
    
    # Group windows by token usage ranges
    token_ranges = {
        'low': [],      # < 200K tokens
        'medium': [],   # 200K - 400K tokens  
        'high': [],     # 400K - 880K tokens
        'extreme': []   # > 880K tokens
    }
    
    for i, window in enumerate(windows):
        start_str = window['start'].strftime('%Y-%m-%d %H:%M')
        end_str = window['end'].strftime('%Y-%m-%d %H:%M')
        duration_str = str(window['duration'])
        tokens = window['total_tokens']
        
        print(f"Window {i+1}: {start_str} → {end_str}")
        print(f"  Duration: {duration_str}")
        print(f"  Tokens: {tokens:,}")
        print(f"  Entries: {len(window['entries'])}")
        
        # Categorize by token usage
        if tokens < 200000:
            token_ranges['low'].append(window)
            print(f"  🟢 Category: LOW (Pro plan safe)")
        elif tokens < 400000:
            token_ranges['medium'].append(window)
            print(f"  🟡 Category: MEDIUM (Max5 plan territory)")
        elif tokens < 880000:
            token_ranges['high'].append(window)
            print(f"  🟠 Category: HIGH (Max20 plan territory)")
        else:
            token_ranges['extreme'].append(window)
            print(f"  🔴 Category: EXTREME (Over Max20 limit!)")
        
        print()
    
    # Summary statistics
    print("📈 USAGE PATTERN ANALYSIS:")
    print("=" * 80)
    print(f"Low usage windows (< 200K tokens): {len(token_ranges['low'])}")
    print(f"Medium usage windows (200K-400K tokens): {len(token_ranges['medium'])}")
    print(f"High usage windows (400K-880K tokens): {len(token_ranges['high'])}")
    print(f"Extreme usage windows (> 880K tokens): {len(token_ranges['extreme'])}")
    
    # Detect likely plan based on highest usage
    max_tokens = max(w['total_tokens'] for w in windows) if windows else 0
    
    print(f"\n🎯 PLAN DETECTION:")
    print("=" * 80)
    print(f"Maximum tokens in any 5-hour window: {max_tokens:,}")
    
    if max_tokens > 880000:
        print("🔴 DETECTED: Need Max20+ plan (or hitting limits)")
        suggested_plan = "max20+"
    elif max_tokens > 400000:
        print("🟠 DETECTED: Max20 plan recommended")
        suggested_plan = "max20"
    elif max_tokens > 200000:
        print("🟡 DETECTED: Max5 plan recommended")
        suggested_plan = "max5"
    else:
        print("🟢 DETECTED: Pro plan sufficient")
        suggested_plan = "pro"
    
    # Look for limit hit patterns
    print(f"\n🚨 LIMIT HIT DETECTION:")
    print("=" * 80)
    
    limit_hits = []
    for window in windows:
        if window['total_tokens'] > 880000:
            limit_hits.append(window)
    
    if limit_hits:
        print(f"Found {len(limit_hits)} windows that likely hit Max20 limits:")
        for hit in limit_hits:
            print(f"  🔴 {hit['start'].strftime('%Y-%m-%d %H:%M')} - {hit['total_tokens']:,} tokens")
    else:
        print("✅ No clear limit hits detected")
    
    return {
        'suggested_plan': suggested_plan,
        'max_tokens': max_tokens,
        'windows': windows,
        'limit_hits': limit_hits
    }

if __name__ == "__main__":
    results = create_sliding_window_detector()
    
    # Save results for implementation
    with open('/workspace/window_detection_results.json', 'w') as f:
        # Convert datetime objects to strings for JSON serialization
        json_results = {
            'suggested_plan': results['suggested_plan'],
            'max_tokens': results['max_tokens'],
            'total_windows': len(results['windows']),
            'limit_hits': len(results['limit_hits'])
        }
        json.dump(json_results, f, indent=2)
    
    print(f"\n💾 Results saved to: /workspace/window_detection_results.json")