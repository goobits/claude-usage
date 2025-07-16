#!/usr/bin/env python3
"""Debug script to find and analyze usage limit messages in Claude JSONL files."""

import json
import os
from pathlib import Path
from datetime import datetime
from collections import defaultdict
import re

# Keywords to search for limit-related messages
LIMIT_KEYWORDS = [
    'usage limit', 'rate limit', 'max usage', 'limit reached', 'exceeded',
    'quota', 'threshold', 'maximum', 'capacity', 'exhausted', 'insufficient',
    'token limit', 'context limit', 'window', 'sliding window', 'cutoff',
    'truncated', 'interrupted', 'stopped', 'halted', 'blocked',
    'daily limit', 'monthly limit', 'usage_limit', 'rate_limit',
    'error', 'warning', 'restriction', 'constraint', 'boundary'
]

# Compile regex patterns for efficient searching
LIMIT_PATTERNS = [re.compile(rf'\b{keyword}\b', re.IGNORECASE) for keyword in LIMIT_KEYWORDS]

class LimitMessageDebugger:
    def __init__(self):
        self.limit_entries = []
        self.error_entries = []
        self.cutoff_patterns = []
        self.token_patterns = defaultdict(list)
        self.message_type_stats = defaultdict(int)
        self.field_stats = defaultdict(set)
        
    def find_jsonl_files(self, base_path='~/.claude'):
        """Find all JSONL files in Claude directories."""
        base_path = Path(base_path).expanduser()
        jsonl_files = []
        
        # Check main projects directory - updated pattern to match actual files
        projects_dir = base_path / 'projects'
        if projects_dir.exists():
            jsonl_files.extend(projects_dir.glob('*/*.jsonl'))  # All JSONL files, not just conversation_*
            
        # Check VM directories
        vms_dir = base_path / 'vms'
        if vms_dir.exists():
            for vm_dir in vms_dir.iterdir():
                if vm_dir.is_dir():
                    vm_projects = vm_dir / 'projects'
                    if vm_projects.exists():
                        jsonl_files.extend(vm_projects.glob('*/*.jsonl'))
                        
        return jsonl_files
    
    def analyze_entry(self, entry, file_path, line_num):
        """Analyze a single JSONL entry for limit-related content."""
        entry_str = json.dumps(entry).lower()
        
        # Track all message types and fields
        if 'type' in entry:
            self.message_type_stats[entry['type']] += 1
            
        # Track unique fields per message type
        msg_type = entry.get('type', 'unknown')
        for field in entry.keys():
            self.field_stats[msg_type].add(field)
        
        # Check for limit keywords in any field
        found_limit = False
        for pattern in LIMIT_PATTERNS:
            if pattern.search(entry_str):
                found_limit = True
                break
                
        # Check for error-related fields
        has_error = any(field in entry for field in ['error', 'errorMessage', 'errorCode', 'status', 'statusCode'])
        
        # Check for high token counts (potential limits)
        total_tokens = 0
        if 'usage' in entry:
            usage = entry['usage']
            if isinstance(usage, dict):
                total_tokens = usage.get('totalTokens', 0)
                if total_tokens > 100000:  # Track high token usage
                    self.token_patterns[f"{total_tokens//1000}k"].append({
                        'file': str(file_path),
                        'line': line_num,
                        'tokens': total_tokens,
                        'type': msg_type,
                        'timestamp': entry.get('timestamp', 'unknown')
                    })
        
        # Store entries with limit keywords or errors
        if found_limit or has_error:
            context = {
                'file': str(file_path),
                'line': line_num,
                'entry': entry,
                'has_limit_keyword': found_limit,
                'has_error_field': has_error,
                'total_tokens': total_tokens
            }
            
            if has_error:
                self.error_entries.append(context)
            if found_limit:
                self.limit_entries.append(context)
                
        # Check for potential cutoff patterns
        if msg_type == 'text' and 'text' in entry:
            text = entry['text']
            if len(text) < 100 and any(phrase in text.lower() for phrase in ['i apologize', 'sorry', 'cut off', 'continue']):
                self.cutoff_patterns.append({
                    'file': str(file_path),
                    'line': line_num,
                    'text': text[:200],
                    'type': msg_type
                })
    
    def scan_files(self):
        """Scan all JSONL files for limit messages."""
        jsonl_files = self.find_jsonl_files()
        
        print(f"Found {len(jsonl_files)} JSONL files to scan...")
        
        total_entries = 0
        for file_path in jsonl_files:
            try:
                with open(file_path, 'r') as f:
                    for line_num, line in enumerate(f, 1):
                        if not line.strip():
                            continue
                        try:
                            entry = json.loads(line)
                            self.analyze_entry(entry, file_path, line_num)
                            total_entries += 1
                        except json.JSONDecodeError:
                            print(f"Error parsing line {line_num} in {file_path}")
            except Exception as e:
                print(f"Error reading {file_path}: {e}")
                
        print(f"\nScanned {total_entries} total entries")
        
    def print_results(self):
        """Print analysis results."""
        print("\n" + "="*80)
        print("LIMIT MESSAGE ANALYSIS RESULTS")
        print("="*80)
        
        # Message type statistics
        print("\n1. MESSAGE TYPE STATISTICS:")
        print("-" * 40)
        for msg_type, count in sorted(self.message_type_stats.items(), key=lambda x: x[1], reverse=True):
            print(f"  {msg_type}: {count}")
            
        # Field statistics per message type
        print("\n2. FIELDS BY MESSAGE TYPE:")
        print("-" * 40)
        for msg_type, fields in sorted(self.field_stats.items()):
            print(f"\n  {msg_type}:")
            print(f"    {', '.join(sorted(fields))}")
        
        # Limit keyword entries
        print(f"\n3. ENTRIES WITH LIMIT KEYWORDS: {len(self.limit_entries)}")
        print("-" * 40)
        for i, context in enumerate(self.limit_entries[:10]):  # Show first 10
            entry = context['entry']
            print(f"\n  Entry {i+1}:")
            print(f"    File: {context['file']}")
            print(f"    Line: {context['line']}")
            print(f"    Type: {entry.get('type', 'unknown')}")
            print(f"    Tokens: {context['total_tokens']}")
            
            # Show relevant fields
            for key, value in entry.items():
                if any(pattern.search(str(value).lower()) for pattern in LIMIT_PATTERNS):
                    print(f"    {key}: {value}")
                    
        # Error entries
        print(f"\n4. ENTRIES WITH ERROR FIELDS: {len(self.error_entries)}")
        print("-" * 40)
        for i, context in enumerate(self.error_entries[:10]):  # Show first 10
            entry = context['entry']
            print(f"\n  Entry {i+1}:")
            print(f"    File: {context['file']}")
            print(f"    Line: {context['line']}")
            print(f"    Type: {entry.get('type', 'unknown')}")
            
            # Show error fields
            for field in ['error', 'errorMessage', 'errorCode', 'status', 'statusCode']:
                if field in entry:
                    print(f"    {field}: {entry[field]}")
                    
        # High token usage patterns
        print(f"\n5. HIGH TOKEN USAGE PATTERNS:")
        print("-" * 40)
        for token_range, instances in sorted(self.token_patterns.items()):
            print(f"\n  {token_range} tokens: {len(instances)} instances")
            for inst in instances[:3]:  # Show first 3
                print(f"    {inst['timestamp']} - {inst['type']} - {inst['tokens']} tokens")
                
        # Potential cutoff patterns
        print(f"\n6. POTENTIAL CUTOFF PATTERNS: {len(self.cutoff_patterns)}")
        print("-" * 40)
        for i, pattern in enumerate(self.cutoff_patterns[:5]):
            print(f"\n  Pattern {i+1}:")
            print(f"    File: {pattern['file']}")
            print(f"    Text: {pattern['text']}")
            
        # Export detailed results
        print("\n7. EXPORTING DETAILED RESULTS...")
        print("-" * 40)
        
        # Export limit entries to file
        if self.limit_entries:
            with open('limit_entries_debug.json', 'w') as f:
                json.dump(self.limit_entries, f, indent=2, default=str)
            print(f"  Exported {len(self.limit_entries)} limit entries to limit_entries_debug.json")
            
        # Export error entries to file
        if self.error_entries:
            with open('error_entries_debug.json', 'w') as f:
                json.dump(self.error_entries, f, indent=2, default=str)
            print(f"  Exported {len(self.error_entries)} error entries to error_entries_debug.json")
            
        # Look for specific patterns in conversations
        print("\n8. CONVERSATION FLOW ANALYSIS:")
        print("-" * 40)
        self.analyze_conversation_flows()
        
    def analyze_conversation_flows(self):
        """Analyze conversation flows for limit patterns."""
        # Group entries by conversation/session
        conversations = defaultdict(list)
        
        jsonl_files = self.find_jsonl_files()
        for file_path in jsonl_files:
            try:
                with open(file_path, 'r') as f:
                    session_entries = []
                    for line in f:
                        if not line.strip():
                            continue
                        try:
                            entry = json.loads(line)
                            session_entries.append(entry)
                        except:
                            pass
                            
                    if session_entries:
                        # Group by conversation ID or file
                        conv_id = str(file_path)
                        conversations[conv_id] = session_entries
            except:
                pass
                
        # Look for conversations that end abruptly
        print(f"  Analyzing {len(conversations)} conversations for cutoff patterns...")
        
        cutoff_conversations = []
        for conv_id, entries in conversations.items():
            if len(entries) < 2:
                continue
                
            # Check last few messages
            last_entries = entries[-5:]
            
            # Look for patterns indicating cutoff
            has_high_tokens = False
            has_error = False
            ends_abruptly = False
            
            for entry in last_entries:
                if 'usage' in entry and isinstance(entry['usage'], dict):
                    tokens = entry['usage'].get('totalTokens', 0)
                    if tokens > 100000:
                        has_high_tokens = True
                        
                if any(field in entry for field in ['error', 'errorMessage']):
                    has_error = True
                    
            # Check if conversation ends without proper closure
            last_entry = entries[-1]
            if last_entry.get('type') == 'text' and 'text' in last_entry:
                text = last_entry['text'].lower()
                if len(text) < 500 and not any(phrase in text for phrase in ['goodbye', 'thank you', 'thanks', 'complete']):
                    ends_abruptly = True
                    
            if has_high_tokens or has_error or ends_abruptly:
                cutoff_conversations.append({
                    'file': conv_id,
                    'total_messages': len(entries),
                    'has_high_tokens': has_high_tokens,
                    'has_error': has_error,
                    'ends_abruptly': ends_abruptly,
                    'last_tokens': entries[-1].get('usage', {}).get('totalTokens', 0) if 'usage' in entries[-1] else 0
                })
                
        print(f"  Found {len(cutoff_conversations)} conversations with potential cutoffs")
        for conv in cutoff_conversations[:5]:
            print(f"    {conv['file'].split('/')[-2]}: {conv['total_messages']} msgs, "
                  f"high_tokens={conv['has_high_tokens']}, error={conv['has_error']}, "
                  f"abrupt={conv['ends_abruptly']}, last_tokens={conv['last_tokens']}")

def main():
    """Main entry point."""
    print("Claude Usage Limit Message Debugger")
    print("===================================\n")
    
    debugger = LimitMessageDebugger()
    debugger.scan_files()
    debugger.print_results()
    
    print("\n\nDebug complete! Check limit_entries_debug.json and error_entries_debug.json for details.")

if __name__ == '__main__':
    main()