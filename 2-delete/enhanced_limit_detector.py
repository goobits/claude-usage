#!/usr/bin/env python3
"""Enhanced script to detect and analyze usage limit patterns in Claude JSONL files."""

import json
import os
from pathlib import Path
from datetime import datetime
from collections import defaultdict
import re

class LimitPatternDetector:
    def __init__(self):
        self.conversations = []
        self.high_token_conversations = []
        self.incomplete_conversations = []
        self.error_patterns = []
        self.usage_analysis = defaultdict(list)
        self.token_thresholds = {
            'high': 100000,      # 100k tokens
            'very_high': 200000, # 200k tokens
            'extreme': 500000    # 500k tokens
        }
        
    def find_jsonl_files(self, base_path='~/.claude'):
        """Find all JSONL files in Claude directories."""
        base_path = Path(base_path).expanduser()
        jsonl_files = []
        
        projects_dir = base_path / 'projects'
        if projects_dir.exists():
            jsonl_files.extend(projects_dir.glob('*/*.jsonl'))
            
        vms_dir = base_path / 'vms'
        if vms_dir.exists():
            for vm_dir in vms_dir.iterdir():
                if vm_dir.is_dir():
                    vm_projects = vm_dir / 'projects'
                    if vm_projects.exists():
                        jsonl_files.extend(vm_projects.glob('*/*.jsonl'))
                        
        return jsonl_files
    
    def analyze_conversation(self, file_path):
        """Analyze a complete conversation for limit patterns."""
        conversation = {
            'file': str(file_path),
            'entries': [],
            'max_tokens': 0,
            'total_input_tokens': 0,
            'total_output_tokens': 0,
            'session_id': None,
            'has_errors': False,
            'incomplete_responses': [],
            'high_token_responses': [],
            'stop_reasons': set(),
            'service_tiers': set(),
            'models': set()
        }
        
        try:
            with open(file_path, 'r') as f:
                for line_num, line in enumerate(f, 1):
                    if not line.strip():
                        continue
                    try:
                        entry = json.loads(line)
                        conversation['entries'].append(entry)
                        
                        # Extract session ID
                        if not conversation['session_id'] and 'sessionId' in entry:
                            conversation['session_id'] = entry['sessionId']
                        
                        # Analyze assistant responses for usage patterns
                        if entry.get('type') == 'assistant':
                            self.analyze_assistant_response(entry, conversation, line_num)
                            
                        # Look for user interruptions or system messages
                        if entry.get('type') == 'user':
                            self.analyze_user_message(entry, conversation, line_num)
                            
                        if entry.get('type') == 'system':
                            self.analyze_system_message(entry, conversation, line_num)
                            
                    except json.JSONDecodeError as e:
                        print(f"JSON error at line {line_num} in {file_path}: {e}")
                        
        except Exception as e:
            print(f"Error reading {file_path}: {e}")
            return None
            
        return conversation
    
    def analyze_assistant_response(self, entry, conversation, line_num):
        """Analyze assistant response for usage and limit indicators."""
        message = entry.get('message', {})
        usage = message.get('usage', {})
        
        if usage:
            # Track token usage
            input_tokens = usage.get('input_tokens', 0) + usage.get('cache_read_input_tokens', 0)
            output_tokens = usage.get('output_tokens', 0)
            total_tokens = input_tokens + output_tokens
            
            conversation['total_input_tokens'] += input_tokens
            conversation['total_output_tokens'] += output_tokens
            conversation['max_tokens'] = max(conversation['max_tokens'], total_tokens)
            
            # Track service tier and model
            if 'service_tier' in usage:
                conversation['service_tiers'].add(usage['service_tier'])
            if 'model' in message:
                conversation['models'].add(message['model'])
            
            # Detect high token usage
            if total_tokens > self.token_thresholds['high']:
                conversation['high_token_responses'].append({
                    'line': line_num,
                    'tokens': total_tokens,
                    'input_tokens': input_tokens,
                    'output_tokens': output_tokens,
                    'usage': usage
                })
                
            # Track in global usage analysis
            self.usage_analysis[conversation['session_id'] or 'unknown'].append({
                'tokens': total_tokens,
                'usage': usage,
                'line': line_num,
                'file': conversation['file']
            })
        
        # Check stop reason for limits
        stop_reason = message.get('stop_reason')
        if stop_reason:
            conversation['stop_reasons'].add(stop_reason)
            
            # Look for limit-related stop reasons
            if stop_reason in ['max_tokens', 'length', 'tool_use_limit', 'error']:
                conversation['incomplete_responses'].append({
                    'line': line_num,
                    'stop_reason': stop_reason,
                    'usage': usage,
                    'content_length': len(str(message.get('content', '')))
                })
        
        # Check for incomplete or truncated content
        content = message.get('content', [])
        if isinstance(content, list):
            for item in content:
                if isinstance(item, dict):
                    text = item.get('text', '') or item.get('thinking', '')
                    if text:
                        # Look for signs of truncation
                        if (len(text) > 1000 and 
                            (text.endswith('...') or 
                             text.endswith('[truncated]') or
                             'I apologize, but I seem to have reached' in text.lower() or
                             'continue in my next response' in text.lower())):
                            conversation['incomplete_responses'].append({
                                'line': line_num,
                                'reason': 'truncated_content',
                                'text_length': len(text),
                                'usage': usage
                            })
    
    def analyze_user_message(self, entry, conversation, line_num):
        """Analyze user messages for interruption patterns."""
        message = entry.get('message', {})
        content = message.get('content', '')
        
        # Handle both string and array content
        if isinstance(content, list):
            content = ' '.join([item.get('text', '') for item in content if isinstance(item, dict)])
        
        # Look for user interruptions
        interrupt_patterns = [
            '[Request interrupted by user]',
            'interrupted',
            'stop generating',
            'please stop',
            'ctrl+c',
            'cancel'
        ]
        
        content_lower = content.lower()
        for pattern in interrupt_patterns:
            if pattern in content_lower:
                conversation['incomplete_responses'].append({
                    'line': line_num,
                    'reason': 'user_interruption',
                    'pattern': pattern,
                    'content': content[:200]  # First 200 chars
                })
                break
    
    def analyze_system_message(self, entry, conversation, line_num):
        """Analyze system messages for error or limit indicators."""
        content = entry.get('content', '')
        level = entry.get('level', '')
        
        # Look for error-related system messages
        error_indicators = ['error', 'limit', 'exceeded', 'quota', 'throttle', 'rate limit']
        
        content_lower = content.lower() if isinstance(content, str) else ''
        for indicator in error_indicators:
            if indicator in content_lower:
                conversation['has_errors'] = True
                self.error_patterns.append({
                    'file': conversation['file'],
                    'line': line_num,
                    'level': level,
                    'content': content,
                    'indicator': indicator
                })
                break
    
    def scan_all_conversations(self):
        """Scan all JSONL files and analyze conversations."""
        jsonl_files = self.find_jsonl_files()
        print(f"Found {len(jsonl_files)} JSONL files to analyze...")
        
        for file_path in jsonl_files:
            conversation = self.analyze_conversation(file_path)
            if conversation:
                self.conversations.append(conversation)
                
                # Categorize conversations
                if conversation['max_tokens'] > self.token_thresholds['high']:
                    self.high_token_conversations.append(conversation)
                    
                if conversation['incomplete_responses'] or conversation['has_errors']:
                    self.incomplete_conversations.append(conversation)
    
    def print_analysis(self):
        """Print comprehensive analysis results."""
        print("\n" + "="*80)
        print("ENHANCED USAGE LIMIT ANALYSIS")
        print("="*80)
        
        print(f"\n📊 OVERALL STATISTICS:")
        print(f"  Total conversations analyzed: {len(self.conversations)}")
        print(f"  High token conversations (>100k): {len(self.high_token_conversations)}")
        print(f"  Conversations with interruptions/errors: {len(self.incomplete_conversations)}")
        print(f"  System error patterns found: {len(self.error_patterns)}")
        
        # Token usage distribution
        print(f"\n📈 TOKEN USAGE DISTRIBUTION:")
        token_ranges = defaultdict(int)
        for conv in self.conversations:
            max_tokens = conv['max_tokens']
            if max_tokens == 0:
                token_ranges['0 tokens'] += 1
            elif max_tokens < 10000:
                token_ranges['<10k tokens'] += 1
            elif max_tokens < 50000:
                token_ranges['10k-50k tokens'] += 1
            elif max_tokens < 100000:
                token_ranges['50k-100k tokens'] += 1
            elif max_tokens < 200000:
                token_ranges['100k-200k tokens'] += 1
            else:
                token_ranges['>200k tokens'] += 1
                
        for range_name, count in sorted(token_ranges.items()):
            print(f"  {range_name}: {count} conversations")
        
        # High token conversations
        if self.high_token_conversations:
            print(f"\n🔥 HIGH TOKEN CONVERSATIONS:")
            for i, conv in enumerate(sorted(self.high_token_conversations, 
                                          key=lambda x: x['max_tokens'], reverse=True)[:10]):
                print(f"\n  #{i+1}: {Path(conv['file']).name}")
                print(f"    Max tokens: {conv['max_tokens']:,}")
                print(f"    Total input: {conv['total_input_tokens']:,}")
                print(f"    Total output: {conv['total_output_tokens']:,}")
                print(f"    Models: {', '.join(conv['models'])}")
                print(f"    Service tiers: {', '.join(conv['service_tiers'])}")
                print(f"    Stop reasons: {', '.join(conv['stop_reasons']) if conv['stop_reasons'] else 'None'}")
                
                if conv['high_token_responses']:
                    print(f"    High token responses: {len(conv['high_token_responses'])}")
                    for resp in conv['high_token_responses'][:3]:  # Show first 3
                        print(f"      Line {resp['line']}: {resp['tokens']:,} tokens")
        
        # Incomplete conversations
        if self.incomplete_conversations:
            print(f"\n⚠️  CONVERSATIONS WITH INTERRUPTIONS/ERRORS:")
            for i, conv in enumerate(self.incomplete_conversations[:10]):
                print(f"\n  #{i+1}: {Path(conv['file']).name}")
                print(f"    Max tokens: {conv['max_tokens']:,}")
                print(f"    Has errors: {conv['has_errors']}")
                
                if conv['incomplete_responses']:
                    print(f"    Interruptions/issues:")
                    for issue in conv['incomplete_responses'][:5]:  # Show first 5
                        reason = issue.get('reason', 'unknown')
                        line = issue.get('line', 'unknown')
                        if reason == 'user_interruption':
                            pattern = issue.get('pattern', '')
                            print(f"      Line {line}: User interruption ({pattern})")
                        elif reason == 'truncated_content':
                            length = issue.get('text_length', 0)
                            print(f"      Line {line}: Truncated content ({length:,} chars)")
                        elif 'stop_reason' in issue:
                            stop_reason = issue['stop_reason']
                            print(f"      Line {line}: Stop reason: {stop_reason}")
        
        # Error patterns
        if self.error_patterns:
            print(f"\n❌ SYSTEM ERROR PATTERNS:")
            for i, error in enumerate(self.error_patterns[:10]):
                print(f"\n  #{i+1}: {Path(error['file']).name}")
                print(f"    Line {error['line']}: {error['level']}")
                print(f"    Indicator: {error['indicator']}")
                print(f"    Content: {error['content'][:200]}...")
        
        # Usage patterns that could indicate limits
        print(f"\n🎯 POTENTIAL LIMIT INDICATORS:")
        
        # Find sessions with sudden drops in token usage
        sessions_with_drops = []
        for session_id, usage_list in self.usage_analysis.items():
            if len(usage_list) < 3:
                continue
                
            # Sort by line number to get chronological order
            usage_list.sort(key=lambda x: x['line'])
            
            # Look for sudden drops in token usage
            for i in range(1, len(usage_list)):
                prev_tokens = usage_list[i-1]['tokens']
                curr_tokens = usage_list[i]['tokens']
                
                # If tokens drop by more than 50% and previous was high
                if (prev_tokens > 50000 and 
                    curr_tokens < prev_tokens * 0.5):
                    sessions_with_drops.append({
                        'session': session_id,
                        'prev_tokens': prev_tokens,
                        'curr_tokens': curr_tokens,
                        'drop_percent': (prev_tokens - curr_tokens) / prev_tokens * 100,
                        'files': list(set([u['file'] for u in usage_list]))
                    })
        
        if sessions_with_drops:
            print(f"  Sessions with sudden token drops: {len(sessions_with_drops)}")
            for drop in sorted(sessions_with_drops, key=lambda x: x['drop_percent'], reverse=True)[:5]:
                print(f"    {drop['session'][:20]}...")
                print(f"      Drop: {drop['prev_tokens']:,} → {drop['curr_tokens']:,} ({drop['drop_percent']:.1f}%)")
        
        # Export detailed data for further analysis
        print(f"\n💾 EXPORTING DETAILED DATA...")
        
        # Export high token conversations
        if self.high_token_conversations:
            with open('high_token_conversations.json', 'w') as f:
                json.dump(self.high_token_conversations, f, indent=2, default=str)
            print(f"  Exported {len(self.high_token_conversations)} high token conversations")
        
        # Export incomplete conversations
        if self.incomplete_conversations:
            with open('incomplete_conversations.json', 'w') as f:
                json.dump(self.incomplete_conversations, f, indent=2, default=str)
            print(f"  Exported {len(self.incomplete_conversations)} incomplete conversations")
        
        # Export usage analysis
        with open('usage_analysis.json', 'w') as f:
            json.dump(dict(self.usage_analysis), f, indent=2, default=str)
        print(f"  Exported usage analysis for {len(self.usage_analysis)} sessions")

def main():
    """Main entry point."""
    print("Enhanced Claude Usage Limit Pattern Detector")
    print("=" * 50)
    
    detector = LimitPatternDetector()
    detector.scan_all_conversations()
    detector.print_analysis()
    
    print("\n✅ Analysis complete!")
    print("Check the exported JSON files for detailed data.")

if __name__ == '__main__':
    main()