#!/usr/bin/env python3
"""
Debug Node.js parsing issues by checking what gets rejected
"""
import json
from pathlib import Path

def analyze_jsonl_parsing(jsonl_file):
    """Analyze what types of entries exist in a JSONL file"""
    print(f"\n🔍 Analyzing: {jsonl_file.name}")
    
    total_lines = 0
    usage_entries = 0
    summary_entries = 0
    other_entries = 0
    invalid_json = 0
    
    entry_types = {}
    
    with open(jsonl_file, 'r') as f:
        for line_num, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue
                
            total_lines += 1
            
            try:
                data = json.loads(line)
                
                # Check entry type
                entry_type = data.get('type', 'no_type')
                entry_types[entry_type] = entry_types.get(entry_type, 0) + 1
                
                # Check if it has usage data
                if 'message' in data and 'usage' in data['message']:
                    usage_entries += 1
                    
                    # Show a few examples of what gets parsed
                    if usage_entries <= 3:
                        usage = data['message']['usage']
                        print(f"  Usage entry {usage_entries}: {usage}")
                        print(f"    Has timestamp: {'timestamp' in data}")
                        print(f"    Has model: {'model' in data.get('message', {})}")
                        print(f"    Has requestId: {'requestId' in data}")
                        
                elif entry_type == 'summary':
                    summary_entries += 1
                else:
                    other_entries += 1
                    if other_entries <= 3:
                        print(f"  Other entry {other_entries}: type={entry_type}, keys={list(data.keys())}")
                        
            except json.JSONDecodeError:
                invalid_json += 1
    
    print(f"\n📊 File summary:")
    print(f"   Total lines: {total_lines}")
    print(f"   Usage entries: {usage_entries}")
    print(f"   Summary entries: {summary_entries}")
    print(f"   Other entries: {other_entries}")
    print(f"   Invalid JSON: {invalid_json}")
    print(f"   Entry types: {entry_types}")
    
    return usage_entries

def main():
    vm_session = Path("/home/miko/.claude/projects/-home-miko-projects-utils-vm")
    
    total_usage_entries = 0
    
    print("=" * 80)
    print("ANALYZING JSONL FILES FOR NODE.JS PARSING ISSUES")
    print("=" * 80)
    
    for jsonl_file in sorted(vm_session.glob('*.jsonl')):
        usage_count = analyze_jsonl_parsing(jsonl_file)
        total_usage_entries += usage_count
    
    print(f"\n📊 TOTAL USAGE ENTRIES FOUND: {total_usage_entries}")
    print("\nExpected Node.js parsing:")
    print("- Should find all usage entries")
    print("- Should skip summary and other entry types")
    print("- Zod schema should validate usage entries")

if __name__ == "__main__":
    main()