#!/usr/bin/env python3
"""
Analyze duplicate entries to understand if deduplication is justified
"""
import json
from pathlib import Path
from collections import defaultdict

def analyze_duplicate_entries(jsonl_file):
    """Analyze what duplicate messageId+requestId pairs actually contain"""
    print(f"\n🔍 Analyzing duplicates in: {jsonl_file.name}")
    
    message_request_groups = defaultdict(list)
    
    with open(jsonl_file, 'r') as f:
        for line_num, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue
                
            try:
                data = json.loads(line)
                
                # Check if it has usage data
                if 'message' in data and 'usage' in data['message']:
                    message_id = data.get('message', {}).get('id')
                    request_id = data.get('requestId')
                    
                    if message_id and request_id:
                        unique_hash = f"{message_id}:{request_id}"
                        
                        entry_info = {
                            'line': line_num,
                            'usage': data['message']['usage'],
                            'timestamp': data.get('timestamp'),
                            'type': data.get('type'),
                            'message_role': data.get('message', {}).get('role'),
                            'message_type': data.get('message', {}).get('type'),
                            'message_model': data.get('message', {}).get('model'),
                            'uuid': data.get('uuid'),
                            'parent_uuid': data.get('parentUuid')
                        }
                        
                        message_request_groups[unique_hash].append(entry_info)
                        
            except json.JSONDecodeError:
                continue
    
    # Analyze duplicate groups
    duplicate_groups = {k: v for k, v in message_request_groups.items() if len(v) > 1}
    
    if not duplicate_groups:
        print("   ✅ No duplicate messageId+requestId pairs found")
        return
    
    print(f"   🔄 Found {len(duplicate_groups)} duplicate groups")
    
    for i, (hash_key, entries) in enumerate(list(duplicate_groups.items())[:3]):  # Show first 3
        print(f"\n   📝 Duplicate group {i+1}: {hash_key}")
        print(f"      Entries: {len(entries)} (lines: {[e['line'] for e in entries]})")
        
        # Check if usage data is identical
        first_usage = entries[0]['usage']
        all_usage_identical = all(entry['usage'] == first_usage for entry in entries)
        
        print(f"      Usage data identical: {all_usage_identical}")
        print(f"      First usage: {first_usage}")
        
        if not all_usage_identical:
            print("      ⚠️  Usage data differs between duplicates:")
            for j, entry in enumerate(entries):
                print(f"         Entry {j+1} (line {entry['line']}): {entry['usage']}")
        
        # Check other fields
        first_timestamp = entries[0]['timestamp']
        timestamps_identical = all(entry['timestamp'] == first_timestamp for entry in entries)
        print(f"      Timestamps identical: {timestamps_identical}")
        
        # Check message details
        print(f"      Message roles: {[e['message_role'] for e in entries]}")
        print(f"      Message types: {[e['message_type'] for e in entries]}")
        print(f"      Entry types: {[e['type'] for e in entries]}")
        print(f"      UUIDs: {[e['uuid'] for e in entries]}")
        
        # Calculate cost difference if usage differs
        if not all_usage_identical:
            # Simple cost calculation for comparison
            costs = []
            for entry in entries:
                usage = entry['usage']
                cost = (usage.get('input_tokens', 0) * 3e-06 + 
                       usage.get('output_tokens', 0) * 1.5e-05 +
                       usage.get('cache_creation_input_tokens', 0) * 3.75e-06 +
                       usage.get('cache_read_input_tokens', 0) * 3e-07)
                costs.append(cost)
            
            print(f"      Estimated costs: {[f'${c:.6f}' for c in costs]}")
            print(f"      Total cost if all counted: ${sum(costs):.6f}")
            print(f"      Cost if deduplicated: ${costs[0]:.6f}")
            print(f"      Potential cost difference: ${sum(costs) - costs[0]:.6f}")

def main():
    vm_session = Path("/home/miko/.claude/projects/-home-miko-projects-utils-vm")
    
    print("=" * 80)
    print("ANALYZING DUPLICATE ENTRIES FOR BILLING JUSTIFICATION")
    print("=" * 80)
    
    # Focus on files with the most duplicates
    high_duplicate_files = [
        "c22b2d3d-3f10-4043-9e8e-ece7ef43db6a.jsonl",  # 174 duplicate pairs
        "fc2297b6-4ad7-4aa1-9586-d23afcf08f87.jsonl",   # 103 duplicate pairs
    ]
    
    for filename in high_duplicate_files:
        jsonl_file = vm_session / filename
        if jsonl_file.exists():
            analyze_duplicate_entries(jsonl_file)

if __name__ == "__main__":
    main()