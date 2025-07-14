#!/usr/bin/env python3
"""
Debug Node.js deduplication logic 
"""
import json
from pathlib import Path
from collections import defaultdict

def analyze_deduplication_impact(jsonl_file):
    """Check what the Node.js deduplication logic would do"""
    print(f"\n🔍 Analyzing deduplication in: {jsonl_file.name}")
    
    usage_entries = []
    message_request_pairs = defaultdict(list)
    
    with open(jsonl_file, 'r') as f:
        for line_num, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue
                
            try:
                data = json.loads(line)
                
                # Check if it has usage data (matches Node.js schema)
                if 'message' in data and 'usage' in data['message']:
                    usage_entries.append({
                        'line': line_num,
                        'usage': data['message']['usage'],
                        'messageId': data.get('message', {}).get('id'),
                        'requestId': data.get('requestId'),
                        'timestamp': data.get('timestamp')
                    })
                    
                    # Track message+request combinations like Node.js does
                    message_id = data.get('message', {}).get('id')
                    request_id = data.get('requestId')
                    
                    if message_id and request_id:
                        unique_hash = f"{message_id}:{request_id}"
                        message_request_pairs[unique_hash].append(line_num)
                        
            except json.JSONDecodeError:
                continue
    
    # Analyze deduplication impact
    total_usage_entries = len(usage_entries)
    duplicate_pairs = {k: v for k, v in message_request_pairs.items() if len(v) > 1}
    entries_with_ids = sum(1 for entry in usage_entries if entry['messageId'] and entry['requestId'])
    entries_without_ids = total_usage_entries - entries_with_ids
    
    print(f"   📊 Usage entries found: {total_usage_entries}")
    print(f"   🔑 Entries with messageId+requestId: {entries_with_ids}")
    print(f"   ❓ Entries without IDs: {entries_without_ids}")
    print(f"   🔄 Duplicate hash pairs: {len(duplicate_pairs)}")
    
    if duplicate_pairs:
        print(f"   ⚠️  Entries that would be deduplicated:")
        for hash_key, line_nums in list(duplicate_pairs.items())[:3]:  # Show first 3
            print(f"      Hash {hash_key}: lines {line_nums} ({len(line_nums)} entries)")
            
        # Calculate how many entries would be removed
        entries_removed = sum(len(lines) - 1 for lines in duplicate_pairs.values())
        print(f"   ❌ Entries that would be REMOVED by deduplication: {entries_removed}")
        print(f"   ✅ Entries that would SURVIVE deduplication: {total_usage_entries - entries_removed}")
        
        return total_usage_entries - entries_removed
    else:
        print(f"   ✅ No duplicates found - all entries would survive")
        return total_usage_entries

def main():
    vm_session = Path("/home/miko/.claude/projects/-home-miko-projects-utils-vm")
    
    print("=" * 80)
    print("ANALYZING NODE.JS DEDUPLICATION IMPACT")
    print("=" * 80)
    
    total_expected_surviving = 0
    
    for jsonl_file in sorted(vm_session.glob('*.jsonl')):
        surviving = analyze_deduplication_impact(jsonl_file)
        total_expected_surviving += surviving
    
    print(f"\n📊 SUMMARY:")
    print(f"   Total entries that should survive Node.js deduplication: {total_expected_surviving}")
    print(f"   Node.js actually reported token counts equivalent to: ~1000-1200 entries")
    print(f"   This suggests additional filtering beyond deduplication!")

if __name__ == "__main__":
    main()