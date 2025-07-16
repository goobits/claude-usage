#!/usr/bin/env python3
"""
Sliding Window Debug Script for Claude Usage Limit Detection

This script analyzes Claude JSONL files to identify patterns that indicate
usage limits have been hit, which can be used to implement auto-detection
of plan-worthy conversations.
"""

import json
import os
from pathlib import Path
from datetime import datetime, timedelta
from collections import defaultdict
import re

class SlidingWindowLimitDetector:
    def __init__(self):
        self.limit_indicators = []
        self.conversation_sessions = defaultdict(list)
        self.token_windows = defaultdict(list)
        self.time_windows = defaultdict(list)
        
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
    
    def parse_timestamp(self, timestamp_str):
        """Parse ISO timestamp string to datetime object."""
        try:
            # Handle 'Z' suffix
            if timestamp_str.endswith('Z'):
                timestamp_str = timestamp_str[:-1] + '+00:00'
            dt = datetime.fromisoformat(timestamp_str)
            # Ensure all datetimes are timezone-aware
            if dt.tzinfo is None:
                from datetime import timezone
                dt = dt.replace(tzinfo=timezone.utc)
            return dt
        except:
            return None
    
    def analyze_conversation_file(self, file_path):
        """Analyze a single conversation file for limit patterns."""
        entries = []
        session_id = None
        
        try:
            with open(file_path, 'r') as f:
                for line_num, line in enumerate(f, 1):
                    if not line.strip():
                        continue
                    try:
                        entry = json.loads(line)
                        entry['_line_num'] = line_num
                        entry['_file'] = str(file_path)
                        
                        # Extract session ID
                        if not session_id and 'sessionId' in entry:
                            session_id = entry['sessionId']
                        
                        entries.append(entry)
                        
                    except json.JSONDecodeError as e:
                        print(f"JSON error at line {line_num} in {file_path}: {e}")
                        
        except Exception as e:
            print(f"Error reading {file_path}: {e}")
            return None
        
        if not entries:
            return None
            
        # Sort entries by timestamp for chronological analysis
        from datetime import timezone
        entries.sort(key=lambda x: self.parse_timestamp(x.get('timestamp', '')) or datetime.min.replace(tzinfo=timezone.utc))
        
        return {
            'session_id': session_id or str(file_path),
            'file_path': str(file_path),
            'entries': entries,
            'total_entries': len(entries)
        }
    
    def detect_limit_patterns(self, conversation):
        """Detect various limit patterns in a conversation."""
        patterns = {
            'system_limit_messages': [],
            'user_interruptions': [],
            'token_limit_hits': [],
            'model_downgrades': [],
            'sudden_drops': [],
            'time_clustering': []
        }
        
        entries = conversation['entries']
        session_id = conversation['session_id']
        
        # Track token usage over time
        token_timeline = []
        assistant_responses = []
        
        for i, entry in enumerate(entries):
            timestamp = self.parse_timestamp(entry.get('timestamp', ''))
            
            # 1. System limit messages
            if (entry.get('type') == 'system' and 
                entry.get('level') == 'warning' and
                'limit' in entry.get('content', '').lower()):
                
                patterns['system_limit_messages'].append({
                    'line': entry['_line_num'],
                    'timestamp': timestamp,
                    'content': entry.get('content', ''),
                    'context': self.get_context_around_entry(entries, i, 3)
                })
            
            # 2. User interruptions
            if entry.get('type') == 'user':
                message = entry.get('message', {})
                content = message.get('content', '')
                
                # Handle array content
                if isinstance(content, list):
                    content = ' '.join([item.get('text', '') for item in content if isinstance(item, dict)])
                
                # Look for interruption patterns
                if any(pattern in content.lower() for pattern in [
                    '[request interrupted by user]',
                    'interrupted',
                    'stop generating',
                    'please stop',
                    'ctrl+c'
                ]):
                    patterns['user_interruptions'].append({
                        'line': entry['_line_num'],
                        'timestamp': timestamp,
                        'content': content,
                        'context': self.get_context_around_entry(entries, i, 2)
                    })
            
            # 3. Assistant responses for token analysis
            if entry.get('type') == 'assistant':
                message = entry.get('message', {})
                usage = message.get('usage', {})
                
                if usage:
                    input_tokens = (usage.get('input_tokens', 0) + 
                                  usage.get('cache_read_input_tokens', 0) + 
                                  usage.get('cache_creation_input_tokens', 0))
                    output_tokens = usage.get('output_tokens', 0)
                    total_tokens = input_tokens + output_tokens
                    
                    assistant_data = {
                        'line': entry['_line_num'],
                        'timestamp': timestamp,
                        'total_tokens': total_tokens,
                        'input_tokens': input_tokens,
                        'output_tokens': output_tokens,
                        'usage': usage,
                        'model': message.get('model', ''),
                        'stop_reason': message.get('stop_reason', ''),
                        'service_tier': usage.get('service_tier', '')
                    }
                    
                    assistant_responses.append(assistant_data)
                    token_timeline.append(assistant_data)
                    
                    # High token usage (potential limit hit)
                    if total_tokens > 100000:
                        patterns['token_limit_hits'].append(assistant_data)
        
        # Analyze token timeline for patterns
        if len(assistant_responses) >= 2:
            patterns['sudden_drops'] = self.find_sudden_drops(assistant_responses)
            patterns['model_downgrades'] = self.find_model_downgrades(assistant_responses)
            patterns['time_clustering'] = self.find_time_clustering(assistant_responses)
        
        return patterns
    
    def get_context_around_entry(self, entries, index, window_size):
        """Get context entries around a specific entry."""
        start = max(0, index - window_size)
        end = min(len(entries), index + window_size + 1)
        
        context = []
        for i in range(start, end):
            entry = entries[i]
            context.append({
                'line': entry['_line_num'],
                'type': entry.get('type', ''),
                'timestamp': entry.get('timestamp', ''),
                'is_target': i == index,
                'summary': self.summarize_entry(entry)
            })
        
        return context
    
    def summarize_entry(self, entry):
        """Create a brief summary of an entry for context."""
        entry_type = entry.get('type', 'unknown')
        
        if entry_type == 'assistant':
            usage = entry.get('message', {}).get('usage', {})
            total_tokens = (usage.get('input_tokens', 0) + 
                          usage.get('output_tokens', 0) + 
                          usage.get('cache_read_input_tokens', 0))
            model = entry.get('message', {}).get('model', 'unknown')
            return f"assistant response: {total_tokens:,} tokens, {model}"
            
        elif entry_type == 'user':
            content = entry.get('message', {}).get('content', '')
            if isinstance(content, list) and content:
                content = content[0].get('text', '')[:50] if content[0] else ''
            else:
                content = str(content)[:50]
            return f"user message: {content}..."
            
        elif entry_type == 'system':
            content = entry.get('content', '')[:50]
            level = entry.get('level', '')
            return f"system {level}: {content}..."
            
        else:
            return f"{entry_type}: {str(entry)[:50]}..."
    
    def find_sudden_drops(self, assistant_responses):
        """Find sudden drops in token usage that might indicate limits."""
        drops = []
        
        for i in range(1, len(assistant_responses)):
            prev = assistant_responses[i-1]
            curr = assistant_responses[i]
            
            prev_tokens = prev['total_tokens']
            curr_tokens = curr['total_tokens']
            
            # Significant drop after high usage
            if (prev_tokens > 50000 and 
                curr_tokens < prev_tokens * 0.5):
                
                drop_percent = (prev_tokens - curr_tokens) / prev_tokens * 100
                drops.append({
                    'prev_line': prev['line'],
                    'curr_line': curr['line'],
                    'prev_tokens': prev_tokens,
                    'curr_tokens': curr_tokens,
                    'drop_percent': drop_percent,
                    'time_gap': self.calculate_time_gap(prev['timestamp'], curr['timestamp']),
                    'prev_model': prev['model'],
                    'curr_model': curr['model']
                })
        
        return drops
    
    def find_model_downgrades(self, assistant_responses):
        """Find model downgrades that might indicate limits."""
        downgrades = []
        
        for i in range(1, len(assistant_responses)):
            prev = assistant_responses[i-1]
            curr = assistant_responses[i]
            
            prev_model = prev['model'].lower()
            curr_model = curr['model'].lower()
            
            # Common downgrade patterns
            if (('opus' in prev_model and 'sonnet' in curr_model) or
                ('4' in prev_model and '3.5' in curr_model)):
                
                downgrades.append({
                    'prev_line': prev['line'],
                    'curr_line': curr['line'],
                    'prev_model': prev['model'],
                    'curr_model': curr['model'],
                    'prev_tokens': prev['total_tokens'],
                    'curr_tokens': curr['total_tokens'],
                    'time_gap': self.calculate_time_gap(prev['timestamp'], curr['timestamp'])
                })
        
        return downgrades
    
    def find_time_clustering(self, assistant_responses):
        """Find time clustering patterns that might indicate limits."""
        clusters = []
        
        # Group responses by time windows (e.g., within 5 minutes)
        time_groups = defaultdict(list)
        
        for response in assistant_responses:
            if response['timestamp']:
                # Round to 5-minute buckets
                rounded_time = response['timestamp'].replace(
                    minute=(response['timestamp'].minute // 5) * 5,
                    second=0,
                    microsecond=0
                )
                time_groups[rounded_time].append(response)
        
        # Look for clusters with high token usage
        for time_bucket, responses in time_groups.items():
            if len(responses) >= 3:  # At least 3 responses in 5 minutes
                total_tokens = sum(r['total_tokens'] for r in responses)
                if total_tokens > 200000:  # High total usage
                    clusters.append({
                        'time_bucket': time_bucket,
                        'response_count': len(responses),
                        'total_tokens': total_tokens,
                        'avg_tokens': total_tokens / len(responses),
                        'responses': responses
                    })
        
        return clusters
    
    def calculate_time_gap(self, timestamp1, timestamp2):
        """Calculate time gap between two timestamps."""
        if not timestamp1 or not timestamp2:
            return None
        try:
            return abs((timestamp2 - timestamp1).total_seconds())
        except:
            return None
    
    def analyze_all_conversations(self):
        """Analyze all conversations for limit patterns."""
        jsonl_files = self.find_jsonl_files()
        print(f"Analyzing {len(jsonl_files)} conversation files...")
        
        all_patterns = []
        
        for file_path in jsonl_files:
            conversation = self.analyze_conversation_file(file_path)
            if conversation:
                patterns = self.detect_limit_patterns(conversation)
                
                # Only include conversations with detected patterns
                if any(patterns.values()):
                    all_patterns.append({
                        'conversation': conversation,
                        'patterns': patterns
                    })
        
        return all_patterns
    
    def print_analysis_report(self, all_patterns):
        """Print comprehensive analysis report."""
        print("\n" + "="*80)
        print("SLIDING WINDOW LIMIT DETECTION ANALYSIS")
        print("="*80)
        
        print(f"\n📊 SUMMARY:")
        print(f"  Conversations with limit patterns: {len(all_patterns)}")
        
        # Count pattern types
        pattern_counts = defaultdict(int)
        for conv_data in all_patterns:
            patterns = conv_data['patterns']
            for pattern_type, instances in patterns.items():
                if instances:
                    pattern_counts[pattern_type] += len(instances)
        
        print(f"  Pattern instances found:")
        for pattern_type, count in sorted(pattern_counts.items()):
            print(f"    {pattern_type}: {count}")
        
        # Detailed analysis by pattern type
        for pattern_type in pattern_counts.keys():
            print(f"\n🔍 {pattern_type.upper().replace('_', ' ')}:")
            print("-" * 50)
            
            for conv_data in all_patterns:
                patterns = conv_data['patterns'][pattern_type]
                if patterns:
                    file_name = Path(conv_data['conversation']['file_path']).name
                    print(f"\n  📁 {file_name}:")
                    
                    for instance in patterns[:3]:  # Show first 3 instances
                        if pattern_type == 'system_limit_messages':
                            print(f"    Line {instance['line']}: {instance['content']}")
                            print(f"    Time: {instance['timestamp']}")
                            
                        elif pattern_type == 'user_interruptions':
                            print(f"    Line {instance['line']}: {instance['content'][:100]}...")
                            print(f"    Time: {instance['timestamp']}")
                            
                        elif pattern_type == 'token_limit_hits':
                            print(f"    Line {instance['line']}: {instance['total_tokens']:,} tokens")
                            print(f"    Model: {instance['model']}, Tier: {instance['service_tier']}")
                            
                        elif pattern_type == 'sudden_drops':
                            print(f"    Lines {instance['prev_line']}-{instance['curr_line']}: "
                                  f"{instance['prev_tokens']:,} → {instance['curr_tokens']:,} "
                                  f"({instance['drop_percent']:.1f}% drop)")
                            if instance['prev_model'] != instance['curr_model']:
                                print(f"    Model change: {instance['prev_model']} → {instance['curr_model']}")
                                
                        elif pattern_type == 'model_downgrades':
                            print(f"    Lines {instance['prev_line']}-{instance['curr_line']}: "
                                  f"{instance['prev_model']} → {instance['curr_model']}")
                            print(f"    Tokens: {instance['prev_tokens']:,} → {instance['curr_tokens']:,}")
                            
                        elif pattern_type == 'time_clustering':
                            print(f"    Time {instance['time_bucket']}: {instance['response_count']} responses")
                            print(f"    Total tokens: {instance['total_tokens']:,}, "
                                  f"Avg: {instance['avg_tokens']:,.0f}")
        
        # Generate sliding window detection rules
        print(f"\n🎯 SLIDING WINDOW DETECTION RULES:")
        print("-" * 50)
        print("Based on the analysis, implement these detection patterns:")
        print()
        print("1. SYSTEM MESSAGE DETECTION:")
        print("   - Type: 'system', Level: 'warning'")
        print("   - Content contains: 'limit reached', 'now using'")
        print("   - Pattern: 'Claude Opus 4 limit reached, now using Sonnet 4'")
        print()
        print("2. USER INTERRUPTION DETECTION:")
        print("   - Type: 'user'")
        print("   - Content contains: '[Request interrupted by user]', 'interrupted'")
        print()
        print("3. TOKEN THRESHOLD DETECTION:")
        print("   - Assistant responses with total_tokens > 100,000")
        print("   - Look for multiple high-token responses in sequence")
        print()
        print("4. SUDDEN DROP DETECTION:")
        print("   - Previous response > 50,000 tokens")
        print("   - Current response < 50% of previous")
        print("   - Consider time gap between responses")
        print()
        print("5. MODEL DOWNGRADE DETECTION:")
        print("   - Model change from Opus to Sonnet")
        print("   - Model change from higher to lower tier")
        print()
        print("6. TIME CLUSTERING DETECTION:")
        print("   - 3+ responses within 5-minute window")
        print("   - Total tokens in window > 200,000")
        
        # Summary for implementation
        print(f"\n💾 IMPLEMENTATION SUMMARY:")
        print(f"  Analyzed {len(all_patterns)} conversations with limit patterns")
        print(f"  Found {sum(len(instances) for conv_data in all_patterns for instances in conv_data['patterns'].values())} total pattern instances")

def main():
    """Main entry point."""
    print("Claude Usage Sliding Window Limit Detector")
    print("=" * 50)
    
    detector = SlidingWindowLimitDetector()
    patterns = detector.analyze_all_conversations()
    detector.print_analysis_report(patterns)
    
    print("\n✅ Analysis complete!")
    print("Use the detection rules above to implement sliding window limit detection.")

if __name__ == '__main__':
    main()