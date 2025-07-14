#!/usr/bin/env python3
"""
Quantify the overall impact of different vs identical duplicates
"""
import json
from pathlib import Path
from collections import defaultdict

def quantify_deduplication_impact(jsonl_file):
    """Calculate the financial impact of deduplication"""
    print(f"\n🔍 Analyzing: {jsonl_file.name}")
    
    message_request_groups = defaultdict(list)
    
    with open(jsonl_file, 'r') as f:
        for line_num, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue
                
            try:
                data = json.loads(line)
                
                if 'message' in data and 'usage' in data['message']:
                    message_id = data.get('message', {}).get('id')
                    request_id = data.get('requestId')
                    
                    if message_id and request_id:
                        unique_hash = f"{message_id}:{request_id}"
                        usage = data['message']['usage']
                        
                        # Calculate cost
                        cost = (usage.get('input_tokens', 0) * 3e-06 + 
                               usage.get('output_tokens', 0) * 1.5e-05 +
                               usage.get('cache_creation_input_tokens', 0) * 3.75e-06 +
                               usage.get('cache_read_input_tokens', 0) * 3e-07)
                        
                        entry_info = {
                            'line': line_num,
                            'usage': usage,
                            'cost': cost
                        }
                        
                        message_request_groups[unique_hash].append(entry_info)
                        
            except json.JSONDecodeError:
                continue
    
    # Analyze duplicate groups
    duplicate_groups = {k: v for k, v in message_request_groups.items() if len(v) > 1}
    single_groups = {k: v for k, v in message_request_groups.items() if len(v) == 1}
    
    total_entries = sum(len(entries) for entries in message_request_groups.values())
    total_cost_all = sum(entry['cost'] for entries in message_request_groups.values() for entry in entries)
    
    # Calculate deduplication impact
    cost_after_dedup = 0
    legitimate_cost_lost = 0
    redundant_cost_removed = 0
    
    identical_groups = 0
    different_groups = 0
    
    for hash_key, entries in duplicate_groups.items():
        # Take first entry cost (what deduplication would keep)
        first_cost = entries[0]['cost']
        cost_after_dedup += first_cost
        
        # Check if all usage data is identical
        first_usage = entries[0]['usage']
        all_identical = all(entry['usage'] == first_usage for entry in entries)
        
        if all_identical:
            # Redundant entries - deduplication is justified
            identical_groups += 1
            total_group_cost = sum(entry['cost'] for entry in entries)
            redundant_cost_removed += (total_group_cost - first_cost)
        else:
            # Different entries - deduplication removes legitimate usage
            different_groups += 1
            total_group_cost = sum(entry['cost'] for entry in entries)
            legitimate_cost_lost += (total_group_cost - first_cost)
    
    # Add single entry costs (no deduplication impact)
    cost_after_dedup += sum(entries[0]['cost'] for entries in single_groups.values())
    
    print(f"   📊 Total entries: {total_entries}")
    print(f"   🔄 Duplicate groups: {len(duplicate_groups)}")
    print(f"   ✅ Groups with identical usage: {identical_groups}")
    print(f"   ⚠️  Groups with different usage: {different_groups}")
    print(f"   💰 Total cost (all entries): ${total_cost_all:.6f}")
    print(f"   💰 Cost after deduplication: ${cost_after_dedup:.6f}")
    print(f"   ✅ Redundant cost correctly removed: ${redundant_cost_removed:.6f}")
    print(f"   ❌ Legitimate cost incorrectly lost: ${legitimate_cost_lost:.6f}")
    print(f"   🎯 Net deduplication accuracy: {(redundant_cost_removed / (redundant_cost_removed + legitimate_cost_lost) * 100):.1f}%" if (redundant_cost_removed + legitimate_cost_lost) > 0 else "N/A")
    
    return {
        'total_cost_all': total_cost_all,
        'cost_after_dedup': cost_after_dedup,
        'redundant_removed': redundant_cost_removed,
        'legitimate_lost': legitimate_cost_lost,
        'identical_groups': identical_groups,
        'different_groups': different_groups
    }

def main():
    vm_session = Path("/home/miko/.claude/projects/-home-miko-projects-utils-vm")
    
    print("=" * 80)
    print("QUANTIFYING DEDUPLICATION FINANCIAL IMPACT")
    print("=" * 80)
    
    total_stats = {
        'total_cost_all': 0,
        'cost_after_dedup': 0,
        'redundant_removed': 0,
        'legitimate_lost': 0,
        'identical_groups': 0,
        'different_groups': 0
    }
    
    for jsonl_file in sorted(vm_session.glob('*.jsonl')):
        if jsonl_file.stat().st_size > 0:  # Skip empty files
            stats = quantify_deduplication_impact(jsonl_file)
            for key in total_stats:
                total_stats[key] += stats[key]
    
    print(f"\n{'='*80}")
    print("OVERALL SESSION IMPACT")
    print("="*80)
    print(f"📊 Total duplicate groups with identical usage: {total_stats['identical_groups']}")
    print(f"⚠️  Total duplicate groups with different usage: {total_stats['different_groups']}")
    print(f"💰 Session total cost (Python version): ${total_stats['total_cost_all']:.2f}")
    print(f"💰 Session cost after deduplication (Node.js): ${total_stats['cost_after_dedup']:.2f}")
    print(f"✅ Correctly removed redundant costs: ${total_stats['redundant_removed']:.2f}")
    print(f"❌ Incorrectly lost legitimate costs: ${total_stats['legitimate_lost']:.2f}")
    
    if total_stats['redundant_removed'] + total_stats['legitimate_lost'] > 0:
        accuracy = total_stats['redundant_removed'] / (total_stats['redundant_removed'] + total_stats['legitimate_lost']) * 100
        print(f"🎯 Deduplication accuracy: {accuracy:.1f}%")
        
        if total_stats['legitimate_lost'] > 0:
            print(f"\n🚨 CONCLUSION: Deduplication removes ${total_stats['legitimate_lost']:.2f} of legitimate billable usage")
            print(f"   This represents {(total_stats['legitimate_lost']/total_stats['total_cost_all']*100):.1f}% of total session cost")

if __name__ == "__main__":
    main()