#!/usr/bin/env python3
"""
Debug script to compare Node.js vs Python parsing on the same session
"""
import json
import subprocess
from pathlib import Path

def count_entries_in_session(session_path):
    """Count total entries and token usage in a specific session directory"""
    session_dir = Path(session_path)
    total_entries = 0
    total_input_tokens = 0
    total_output_tokens = 0
    total_cache_creation = 0
    total_cache_read = 0
    
    print(f"\n🔍 Analyzing session: {session_dir.name}")
    
    for jsonl_file in session_dir.glob('*.jsonl'):
        print(f"  📄 File: {jsonl_file.name}")
        file_entries = 0
        file_input = 0
        file_output = 0
        file_cache_creation = 0
        file_cache_read = 0
        
        with open(jsonl_file, 'r') as f:
            for line_num, line in enumerate(f, 1):
                line = line.strip()
                if not line:
                    continue
                    
                try:
                    data = json.loads(line)
                    
                    # Check if this is a usage entry
                    if 'message' in data and 'usage' in data['message']:
                        usage = data['message']['usage']
                        file_entries += 1
                        file_input += usage.get('input_tokens', 0)
                        file_output += usage.get('output_tokens', 0)
                        file_cache_creation += usage.get('cache_creation_input_tokens', 0)
                        file_cache_read += usage.get('cache_read_input_tokens', 0)
                        
                        if file_entries <= 3:  # Show first 3 entries
                            print(f"    Line {line_num}: {usage}")
                            
                except json.JSONDecodeError:
                    continue
        
        print(f"    💰 File totals: {file_entries} entries, input={file_input}, output={file_output}, cache_creation={file_cache_creation}, cache_read={file_cache_read}")
        total_entries += file_entries
        total_input_tokens += file_input
        total_output_tokens += file_output
        total_cache_creation += file_cache_creation
        total_cache_read += file_cache_read
    
    print(f"\n📊 Session totals:")
    print(f"   Total entries: {total_entries}")
    print(f"   Input tokens: {total_input_tokens:,}")
    print(f"   Output tokens: {total_output_tokens:,}")
    print(f"   Cache creation: {total_cache_creation:,}")
    print(f"   Cache read: {total_cache_read:,}")
    
    return {
        'entries': total_entries,
        'input_tokens': total_input_tokens,
        'output_tokens': total_output_tokens,
        'cache_creation_tokens': total_cache_creation,
        'cache_read_tokens': total_cache_read
    }

def get_nodejs_session_data():
    """Get session data from Node.js version"""
    try:
        result = subprocess.run(['claude-usage', 'session', '--json'], 
                              capture_output=True, text=True, check=True)
        data = json.loads(result.stdout)
        return data.get('session', [])
    except Exception as e:
        print(f"Error getting Node.js data: {e}")
        return []

def get_python_session_data():
    """Get session data from Python version"""
    try:
        result = subprocess.run(['cusage', 'session', '--json'], 
                              capture_output=True, text=True, check=True)
        data = json.loads(result.stdout)
        return data.get('session', [])
    except Exception as e:
        print(f"Error getting Python data: {e}")
        return []

def main():
    # Test specific session
    vm_session = "/home/miko/.claude/projects/-home-miko-projects-utils-vm"
    
    print("=" * 80)
    print("MANUAL PARSING ANALYSIS")
    print("=" * 80)
    manual_data = count_entries_in_session(vm_session)
    
    print("\n" + "=" * 80)
    print("NODE.JS VERSION RESULTS")
    print("=" * 80)
    nodejs_sessions = get_nodejs_session_data()
    vm_nodejs = None
    for session in nodejs_sessions:
        if 'vm' in session.get('sessionId', '') or 'vm' in session.get('projectPath', ''):
            vm_nodejs = session
            print(f"Found VM session: {session['sessionId']}")
            print(f"  Input tokens: {session.get('inputTokens', 0):,}")
            print(f"  Output tokens: {session.get('outputTokens', 0):,}")
            print(f"  Cache creation: {session.get('cacheCreationTokens', 0):,}")
            print(f"  Cache read: {session.get('cacheReadTokens', 0):,}")
            print(f"  Total cost: ${session.get('totalCost', 0):.2f}")
            break
    
    if not vm_nodejs:
        print("❌ No VM session found in Node.js output")
    
    print("\n" + "=" * 80)
    print("PYTHON VERSION RESULTS")  
    print("=" * 80)
    python_sessions = get_python_session_data()
    vm_python = None
    for session in python_sessions:
        if 'vm' in session.get('sessionId', '') or 'vm' in session.get('projectPath', ''):
            vm_python = session
            print(f"Found VM session: {session['sessionId']}")
            print(f"  Input tokens: {session.get('inputTokens', 0):,}")
            print(f"  Output tokens: {session.get('outputTokens', 0):,}")
            print(f"  Cache creation: {session.get('cacheCreationTokens', 0):,}")
            print(f"  Cache read: {session.get('cacheReadTokens', 0):,}")
            print(f"  Total cost: ${session.get('totalCost', 0):.2f}")
            break
    
    if not vm_python:
        print("❌ No VM session found in Python output")
    
    print("\n" + "=" * 80)
    print("COMPARISON ANALYSIS")
    print("=" * 80)
    
    if manual_data and vm_nodejs and vm_python:
        print(f"{'Metric':<20} {'Manual':<15} {'Node.js':<15} {'Python':<15} {'Node.js Match':<15} {'Python Match':<15}")
        print("-" * 100)
        
        metrics = [
            ('Input tokens', 'input_tokens', 'inputTokens'),
            ('Output tokens', 'output_tokens', 'outputTokens'),  
            ('Cache creation', 'cache_creation_tokens', 'cacheCreationTokens'),
            ('Cache read', 'cache_read_tokens', 'cacheReadTokens')
        ]
        
        for name, manual_key, tool_key in metrics:
            manual_val = manual_data[manual_key]
            nodejs_val = vm_nodejs.get(tool_key, 0)
            python_val = vm_python.get(tool_key, 0)
            nodejs_match = "✅" if manual_val == nodejs_val else "❌"
            python_match = "✅" if manual_val == python_val else "❌"
            
            print(f"{name:<20} {manual_val:<15,} {nodejs_val:<15,} {python_val:<15,} {nodejs_match:<15} {python_match:<15}")

if __name__ == "__main__":
    main()