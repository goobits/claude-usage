#!/usr/bin/env python3
"""
Sliding window plan detector based on actual Claude usage patterns
"""
import json
from pathlib import Path
from datetime import datetime, timedelta

def create_plan_detector():
    """Create auto-plan detection based on sliding 5-hour windows"""
    
    # Analyze the actual files we have
    files = [
        "/home/developer/.claude/projects/-workspace/f6bc4859-f162-4d72-9159-a39ef5d9594a.jsonl",
        "/home/developer/.claude/projects/-workspace/b2355a52-4b6d-47f1-a5a9-f6d786aaae6a.jsonl"
    ]
    
    all_usage_entries = []
    
    for file_path in files:
        if not Path(file_path).exists():
            continue
        
        print(f"📁 Processing {Path(file_path).name}")
        
        try:
            with open(file_path, 'r') as f:
                for line_num, line in enumerate(f, 1):
                    try:
                        entry = json.loads(line.strip())
                        
                        # Skip summaries
                        if entry.get('type') == 'summary':
                            continue
                        
                        # Get usage data
                        usage = entry.get('message', {}).get('usage', {})
                        timestamp = entry.get('timestamp')
                        
                        if usage and timestamp:
                            try:
                                dt = datetime.fromisoformat(timestamp.replace('Z', '+00:00'))
                                
                                total_tokens = (usage.get('input_tokens', 0) + 
                                              usage.get('output_tokens', 0) + 
                                              usage.get('cache_creation_input_tokens', 0) + 
                                              usage.get('cache_read_input_tokens', 0))
                                
                                if total_tokens > 0:
                                    all_usage_entries.append({
                                        'timestamp': dt,
                                        'tokens': total_tokens,
                                        'usage_breakdown': usage,
                                        'model': entry.get('message', {}).get('model'),
                                        'file': Path(file_path).name,
                                        'line': line_num
                                    })
                            except:
                                continue
                    except json.JSONDecodeError:
                        continue
        except Exception as e:
            print(f"❌ Error reading {file_path}: {e}")
    
    # Sort all entries by timestamp
    all_usage_entries.sort(key=lambda x: x['timestamp'])
    
    print(f"📊 Total usage entries: {len(all_usage_entries)}")
    
    # Create 5-hour sliding windows
    windows = create_5_hour_windows(all_usage_entries)
    
    # Detect plan based on highest usage window
    plan_detection = analyze_windows_for_plan(windows)
    
    # Look for limit hit patterns
    limit_analysis = analyze_limit_patterns(all_usage_entries, windows)
    
    return {
        'windows': windows,
        'plan_detection': plan_detection,
        'limit_analysis': limit_analysis
    }

def create_5_hour_windows(entries):
    """Create 5-hour sliding windows from usage entries"""
    
    if not entries:
        return []
    
    windows = []
    
    # Find natural session breaks (>5 hour gaps)
    session_breaks = [0]  # Start of first session
    
    for i in range(1, len(entries)):
        gap = entries[i]['timestamp'] - entries[i-1]['timestamp']
        if gap > timedelta(hours=5):
            session_breaks.append(i)
    
    session_breaks.append(len(entries))  # End of last session
    
    # Create windows for each session
    for i in range(len(session_breaks) - 1):
        start_idx = session_breaks[i]
        end_idx = session_breaks[i + 1]
        session_entries = entries[start_idx:end_idx]
        
        if not session_entries:
            continue
        
        # For now, treat each natural session as one window
        # (could be enhanced to split long sessions into 5-hour chunks)
        window = {
            'start_time': session_entries[0]['timestamp'],
            'end_time': session_entries[-1]['timestamp'],
            'duration': session_entries[-1]['timestamp'] - session_entries[0]['timestamp'],
            'entries': session_entries,
            'total_tokens': sum(e['tokens'] for e in session_entries),
            'message_count': len(session_entries)
        }
        
        windows.append(window)
    
    return windows

def analyze_windows_for_plan(windows):
    """Analyze windows to detect the likely Claude plan"""
    
    if not windows:
        return {'detected_plan': 'unknown', 'confidence': 'none'}
    
    max_tokens = max(w['total_tokens'] for w in windows)
    
    print(f"\n🎯 PLAN DETECTION ANALYSIS:")
    print("=" * 50)
    
    # Analyze each window
    for i, window in enumerate(windows, 1):
        duration_str = str(window['duration']).split('.')[0]  # Remove microseconds
        
        print(f"Window {i}: {window['start_time'].strftime('%Y-%m-%d %H:%M')} → {window['end_time'].strftime('%H:%M')}")
        print(f"  Duration: {duration_str}")
        print(f"  Tokens: {window['total_tokens']:,}")
        print(f"  Messages: {window['message_count']}")
        
        # Plan category
        if window['total_tokens'] > 880000:
            print(f"  🔴 EXCEEDS Max20 limit (880K)")
        elif window['total_tokens'] > 400000:
            print(f"  🟠 Max20 plan territory (400K-880K)")
        elif window['total_tokens'] > 200000:
            print(f"  🟡 Max5 plan territory (200K-400K)")
        else:
            print(f"  🟢 Pro plan safe (<200K)")
        
        print()
    
    # Determine overall plan recommendation
    if max_tokens > 880000:
        detected_plan = "max20+"
        confidence = "high"
        print(f"🚨 RECOMMENDATION: Need Max20+ plan or hitting limits")
        print(f"   Highest usage: {max_tokens:,} tokens (exceeds Max20 880K limit)")
    elif max_tokens > 400000:
        detected_plan = "max20"
        confidence = "high"
        print(f"🟠 RECOMMENDATION: Max20 plan")
        print(f"   Highest usage: {max_tokens:,} tokens (within Max20 880K limit)")
    elif max_tokens > 200000:
        detected_plan = "max5"
        confidence = "medium"
        print(f"🟡 RECOMMENDATION: Max5 plan")
        print(f"   Highest usage: {max_tokens:,} tokens (within Max5 400K limit)")
    else:
        detected_plan = "pro"
        confidence = "medium"
        print(f"🟢 RECOMMENDATION: Pro plan sufficient")
        print(f"   Highest usage: {max_tokens:,} tokens (within Pro 200K limit)")
    
    return {
        'detected_plan': detected_plan,
        'confidence': confidence,
        'max_tokens': max_tokens,
        'total_windows': len(windows)
    }

def analyze_limit_patterns(entries, windows):
    """Look for patterns that indicate hitting usage limits"""
    
    print(f"\n🚨 LIMIT HIT ANALYSIS:")
    print("=" * 50)
    
    # Look for user interruptions near high usage
    interruptions = []
    for entry in entries:
        # We already identified interruptions in the simple analysis
        pass
    
    # Look for windows that likely hit limits
    potential_limit_hits = []
    for window in windows:
        # Very high token usage suggests hitting limits
        if window['total_tokens'] > 800000:  # Close to Max20 limit
            potential_limit_hits.append(window)
        
        # Very long duration might indicate hitting limits and waiting
        if window['duration'] > timedelta(hours=4):
            print(f"⚠️  Long session detected: {window['duration']} - might indicate limit constraints")
    
    if potential_limit_hits:
        print(f"🔴 Found {len(potential_limit_hits)} potential limit hits:")
        for hit in potential_limit_hits:
            print(f"  {hit['start_time'].strftime('%Y-%m-%d %H:%M')} - {hit['total_tokens']:,} tokens")
    else:
        print("✅ No clear limit hits detected in current data")
    
    # Implementation strategy for real-time detection
    print(f"\n💡 IMPLEMENTATION STRATEGY:")
    print("=" * 50)
    print("1. Track usage in 5-hour rolling windows")
    print("2. Detect plan based on highest window usage:")
    print("   - >880K tokens = Max20+ (or hitting limits)")
    print("   - 400K-880K tokens = Max20 plan")
    print("   - 200K-400K tokens = Max5 plan")
    print("   - <200K tokens = Pro plan")
    print("3. Cache detected plan and update on new limit evidence")
    print("4. Look for user interruptions + high usage as limit indicators")
    
    return {
        'potential_limit_hits': len(potential_limit_hits),
        'long_sessions': sum(1 for w in windows if w['duration'] > timedelta(hours=4))
    }

if __name__ == "__main__":
    print("🔍 CLAUDE USAGE PLAN DETECTOR")
    print("=" * 50)
    
    results = create_plan_detector()
    
    # Save configuration for implementation
    config = {
        'detected_plan': results['plan_detection']['detected_plan'],
        'confidence': results['plan_detection']['confidence'],
        'max_tokens_observed': results['plan_detection']['max_tokens'],
        'detection_timestamp': datetime.now().isoformat(),
        'token_limits': {
            'pro': 200000,
            'max5': 400000,
            'max20': 880000
        },
        'window_duration_hours': 5
    }
    
    with open('/workspace/claude_plan_config.json', 'w') as f:
        json.dump(config, f, indent=2)
    
    print(f"\n💾 Plan configuration saved to: /workspace/claude_plan_config.json")
    print(f"🎯 Detected plan: {config['detected_plan']} ({config['confidence']} confidence)")