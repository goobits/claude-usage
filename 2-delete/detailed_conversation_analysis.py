#!/usr/bin/env python3
"""
Detailed Conversation Analysis for 5-Hour Window Implementation
Focus on message timing patterns, gaps, and conversation structure
"""
import os
import json
import glob
from pathlib import Path
from datetime import datetime, timedelta
from collections import defaultdict, Counter
import statistics
# Visualization libraries removed for minimal dependencies

class DetailedConversationAnalyzer:
    def __init__(self):
        self.home_dir = Path.home()
        
    def discover_claude_paths(self):
        """Find all Claude instances"""
        paths = set()
        
        # Main Claude path
        main_path = self.home_dir / '.claude'
        if (main_path / 'projects').exists():
            paths.add(str(main_path))
        
        # VM paths
        vms_dir = main_path / 'vms'
        if vms_dir.exists():
            try:
                for vm_name in os.listdir(vms_dir):
                    vm_path = vms_dir / vm_name
                    if vm_path.is_dir() and (vm_path / 'projects').exists():
                        paths.add(str(vm_path))
            except (OSError, PermissionError):
                pass
                
        return list(paths)
    
    def parse_timestamp(self, timestamp_str):
        """Parse ISO timestamp into datetime object"""
        try:
            timestamp = timestamp_str
            if timestamp.endswith('Z'):
                timestamp = timestamp[:-1] + '+00:00'
            date = datetime.fromisoformat(timestamp)
            # Convert to naive datetime for comparison
            if date.tzinfo is not None:
                date = date.replace(tzinfo=None)
            return date
        except:
            return None
    
    def extract_message_data(self, data):
        """Extract relevant message data for analysis"""
        if not ('message' in data and 'usage' in data['message']):
            return None
            
        usage = data['message']['usage']
        
        return {
            'session_id': data.get('sessionId'),
            'message_id': data['message'].get('id'),
            'role': data['message'].get('role'),
            'model': data['message'].get('model'),
            'timestamp': self.parse_timestamp(data['timestamp']) if 'timestamp' in data else None,
            'input_tokens': usage.get('input_tokens', 0),
            'output_tokens': usage.get('output_tokens', 0),
            'cache_creation_tokens': usage.get('cache_creation_input_tokens', 0),
            'cache_read_tokens': usage.get('cache_read_input_tokens', 0),
            'total_tokens': (usage.get('input_tokens', 0) + 
                           usage.get('output_tokens', 0) + 
                           usage.get('cache_creation_input_tokens', 0) + 
                           usage.get('cache_read_input_tokens', 0)),
            'cost_usd': data.get('costUSD', 0),
            'request_id': data.get('requestId'),
            'uuid': data.get('uuid'),
            'parent_uuid': data.get('parentUuid')
        }
    
    def analyze_detailed_conversations(self):
        """Deep analysis of conversation patterns"""
        paths = self.discover_claude_paths()
        print(f"🔍 Analyzing conversation patterns across {len(paths)} Claude instances...")
        
        # Track conversations by session_id
        conversations = defaultdict(list)  # session_id -> list of message_data
        all_messages = []
        
        total_files = 0
        total_entries = 0
        usage_entries = 0
        
        # Collect all conversation data
        for claude_path in paths:
            try:
                projects_dir = Path(claude_path) / 'projects'
                if not projects_dir.exists():
                    continue
                    
                for session_dir in projects_dir.iterdir():
                    if not session_dir.is_dir():
                        continue
                    
                    jsonl_files = list(session_dir.glob('*.jsonl'))
                    total_files += len(jsonl_files)
                    
                    for jsonl_file in jsonl_files:
                        try:
                            with open(jsonl_file, 'r') as f:
                                for line in f:
                                    line = line.strip()
                                    if not line:
                                        continue
                                    
                                    try:
                                        data = json.loads(line)
                                        total_entries += 1
                                        
                                        message_data = self.extract_message_data(data)
                                        if message_data:
                                            usage_entries += 1
                                            session_id = message_data['session_id']
                                            conversations[session_id].append(message_data)
                                            all_messages.append(message_data)
                                            
                                    except json.JSONDecodeError:
                                        continue
                                        
                        except Exception as e:
                            print(f"❌ Error processing {jsonl_file}: {e}")
                            continue
                            
            except Exception as e:
                print(f"❌ Error processing {claude_path}: {e}")
                continue
        
        print(f"📁 Processed {total_files} files with {total_entries} total entries")
        print(f"💬 Found {usage_entries} usage entries across {len(conversations)} conversations")
        print()
        
        # Sort messages within each conversation by timestamp
        for session_id in conversations:
            conversations[session_id].sort(key=lambda x: x['timestamp'] if x['timestamp'] else datetime.min)
        
        return self.analyze_conversation_timing_patterns(conversations, all_messages)
    
    def analyze_conversation_timing_patterns(self, conversations, all_messages):
        """Analyze timing patterns within conversations"""
        analysis = {
            'conversation_stats': [],
            'timing_patterns': {},
            'gap_analysis': {},
            'token_flow_analysis': {},
            'session_boundaries': {},
            'five_hour_window_analysis': {}
        }
        
        print("🕒 TIMING PATTERN ANALYSIS")
        print("="*60)
        
        # Analyze each conversation
        for session_id, messages in conversations.items():
            if len(messages) < 2:
                continue
                
            conv_analysis = self.analyze_single_conversation(session_id, messages)
            analysis['conversation_stats'].append(conv_analysis)
        
        # Global timing analysis
        if analysis['conversation_stats']:
            analysis['timing_patterns'] = self.analyze_global_timing_patterns(analysis['conversation_stats'])
            analysis['gap_analysis'] = self.analyze_conversation_gaps(analysis['conversation_stats'])
            analysis['token_flow_analysis'] = self.analyze_token_flow_patterns(analysis['conversation_stats'])
            analysis['five_hour_window_analysis'] = self.analyze_five_hour_windows(analysis['conversation_stats'])
        
        return analysis
    
    def analyze_single_conversation(self, session_id, messages):
        """Analyze timing patterns within a single conversation"""
        if len(messages) < 2:
            return None
            
        start_time = messages[0]['timestamp']
        end_time = messages[-1]['timestamp']
        duration_hours = (end_time - start_time).total_seconds() / 3600
        
        # Calculate message intervals
        intervals = []
        gaps_over_1h = []
        gaps_over_30min = []
        gaps_over_5min = []
        
        for i in range(1, len(messages)):
            prev_time = messages[i-1]['timestamp']
            curr_time = messages[i]['timestamp']
            interval_minutes = (curr_time - prev_time).total_seconds() / 60
            intervals.append(interval_minutes)
            
            if interval_minutes > 60:  # > 1 hour
                gaps_over_1h.append({
                    'duration_hours': interval_minutes / 60,
                    'before_msg': i-1,
                    'after_msg': i,
                    'before_time': prev_time,
                    'after_time': curr_time
                })
            elif interval_minutes > 30:  # > 30 minutes
                gaps_over_30min.append({
                    'duration_minutes': interval_minutes,
                    'before_msg': i-1,
                    'after_msg': i
                })
            elif interval_minutes > 5:  # > 5 minutes
                gaps_over_5min.append({
                    'duration_minutes': interval_minutes,
                    'before_msg': i-1,
                    'after_msg': i
                })
        
        # Token flow analysis
        total_tokens = sum(msg['total_tokens'] for msg in messages)
        tokens_per_hour = total_tokens / max(duration_hours, 0.017)  # min 1 minute
        
        # Role distribution
        role_counts = Counter(msg['role'] for msg in messages)
        
        # Model usage
        models_used = set(msg['model'] for msg in messages if msg['model'])
        
        # Burst analysis (periods of high activity)
        bursts = self.identify_conversation_bursts(messages, intervals)
        
        return {
            'session_id': session_id,
            'duration_hours': duration_hours,
            'message_count': len(messages),
            'start_time': start_time,
            'end_time': end_time,
            'message_intervals': intervals,
            'gaps_over_1h': gaps_over_1h,
            'gaps_over_30min': gaps_over_30min,
            'gaps_over_5min': gaps_over_5min,
            'total_tokens': total_tokens,
            'tokens_per_hour': tokens_per_hour,
            'role_distribution': dict(role_counts),
            'models_used': list(models_used),
            'bursts': bursts,
            'avg_interval_minutes': statistics.mean(intervals) if intervals else 0,
            'median_interval_minutes': statistics.median(intervals) if intervals else 0,
            'max_gap_hours': max([g['duration_hours'] for g in gaps_over_1h]) if gaps_over_1h else 0,
            'natural_break_points': len(gaps_over_1h)  # Number of 1+ hour gaps
        }
    
    def identify_conversation_bursts(self, messages, intervals):
        """Identify bursts of high activity within conversations"""
        bursts = []
        if not intervals:
            return bursts
            
        # Define burst as period with message intervals < 2 minutes for 5+ consecutive messages
        current_burst = []
        
        for i, interval in enumerate(intervals):
            if interval < 2:  # < 2 minutes between messages
                if not current_burst:
                    current_burst = [i]  # Start new burst
                current_burst.append(i + 1)
            else:
                if len(current_burst) >= 5:  # End burst if it has 5+ messages
                    start_time = messages[current_burst[0]]['timestamp']
                    end_time = messages[current_burst[-1]]['timestamp']
                    bursts.append({
                        'start_msg_idx': current_burst[0],
                        'end_msg_idx': current_burst[-1],
                        'message_count': len(current_burst),
                        'duration_minutes': (end_time - start_time).total_seconds() / 60,
                        'start_time': start_time,
                        'end_time': end_time
                    })
                current_burst = []
        
        # Check final burst
        if len(current_burst) >= 5:
            start_time = messages[current_burst[0]]['timestamp']
            end_time = messages[current_burst[-1]]['timestamp']
            bursts.append({
                'start_msg_idx': current_burst[0],
                'end_msg_idx': current_burst[-1],
                'message_count': len(current_burst),
                'duration_minutes': (end_time - start_time).total_seconds() / 60,
                'start_time': start_time,
                'end_time': end_time
            })
        
        return bursts
    
    def analyze_global_timing_patterns(self, conversation_stats):
        """Analyze timing patterns across all conversations"""
        all_intervals = []
        for conv in conversation_stats:
            all_intervals.extend(conv['message_intervals'])
        
        if not all_intervals:
            return {}
            
        return {
            'total_message_intervals': len(all_intervals),
            'avg_interval_minutes': statistics.mean(all_intervals),
            'median_interval_minutes': statistics.median(all_intervals),
            'interval_distribution': {
                '< 1 minute': len([i for i in all_intervals if i < 1]),
                '1-5 minutes': len([i for i in all_intervals if 1 <= i < 5]),
                '5-30 minutes': len([i for i in all_intervals if 5 <= i < 30]),
                '30-60 minutes': len([i for i in all_intervals if 30 <= i < 60]),
                '1-6 hours': len([i for i in all_intervals if 60 <= i < 360]),
                '> 6 hours': len([i for i in all_intervals if i >= 360])
            }
        }
    
    def analyze_conversation_gaps(self, conversation_stats):
        """Analyze gaps that could indicate natural session breaks"""
        all_1h_gaps = []
        all_30min_gaps = []
        
        for conv in conversation_stats:
            all_1h_gaps.extend(conv['gaps_over_1h'])
            all_30min_gaps.extend(conv['gaps_over_30min'])
        
        return {
            'total_1h_gaps': len(all_1h_gaps),
            'total_30min_gaps': len(all_30min_gaps),
            'conversations_with_1h_gaps': len([c for c in conversation_stats if c['gaps_over_1h']]),
            'avg_gap_duration_hours': statistics.mean([g['duration_hours'] for g in all_1h_gaps]) if all_1h_gaps else 0,
            'max_gap_duration_hours': max([g['duration_hours'] for g in all_1h_gaps]) if all_1h_gaps else 0,
            'gap_distribution': {
                '1-2 hours': len([g for g in all_1h_gaps if 1 <= g['duration_hours'] < 2]),
                '2-5 hours': len([g for g in all_1h_gaps if 2 <= g['duration_hours'] < 5]),
                '5-12 hours': len([g for g in all_1h_gaps if 5 <= g['duration_hours'] < 12]),
                '> 12 hours': len([g for g in all_1h_gaps if g['duration_hours'] >= 12])
            }
        }
    
    def analyze_token_flow_patterns(self, conversation_stats):
        """Analyze token consumption patterns"""
        durations = [c['duration_hours'] for c in conversation_stats]
        token_rates = [c['tokens_per_hour'] for c in conversation_stats]
        
        return {
            'avg_tokens_per_hour': statistics.mean(token_rates) if token_rates else 0,
            'median_tokens_per_hour': statistics.median(token_rates) if token_rates else 0,
            'max_tokens_per_hour': max(token_rates) if token_rates else 0,
            'token_rate_distribution': {
                '< 100 tok/h': len([r for r in token_rates if r < 100]),
                '100-500 tok/h': len([r for r in token_rates if 100 <= r < 500]),
                '500-1000 tok/h': len([r for r in token_rates if 500 <= r < 1000]),
                '> 1000 tok/h': len([r for r in token_rates if r >= 1000])
            }
        }
    
    def analyze_five_hour_windows(self, conversation_stats):
        """Analyze implications for 5-hour window implementation"""
        long_conversations = [c for c in conversation_stats if c['duration_hours'] > 5]
        conversations_with_breaks = [c for c in conversation_stats if c['natural_break_points'] > 0]
        
        # Simulate 5-hour window breaks
        window_breaks_needed = 0
        total_natural_breaks = 0
        
        for conv in conversation_stats:
            if conv['duration_hours'] > 5:
                # How many 5-hour windows would this conversation span?
                windows_needed = int(conv['duration_hours'] / 5) + 1
                window_breaks_needed += (windows_needed - 1)
                
                # How many natural breaks (1+ hour gaps) does it have?
                total_natural_breaks += conv['natural_break_points']
        
        return {
            'conversations_over_5h': len(long_conversations),
            'percentage_over_5h': len(long_conversations) / len(conversation_stats) * 100 if conversation_stats else 0,
            'conversations_with_natural_breaks': len(conversations_with_breaks),
            'percentage_with_natural_breaks': len(conversations_with_breaks) / len(conversation_stats) * 100 if conversation_stats else 0,
            'window_breaks_needed': window_breaks_needed,
            'natural_breaks_available': total_natural_breaks,
            'break_alignment_ratio': total_natural_breaks / max(window_breaks_needed, 1),
            'longest_conversation_hours': max([c['duration_hours'] for c in conversation_stats]) if conversation_stats else 0,
            'avg_natural_breaks_per_conversation': total_natural_breaks / len(conversation_stats) if conversation_stats else 0
        }
    
    def display_detailed_analysis(self, analysis):
        """Display comprehensive conversation timing analysis"""
        stats = analysis['conversation_stats']
        timing = analysis['timing_patterns']
        gaps = analysis['gap_analysis']
        tokens = analysis['token_flow_analysis']
        five_hour = analysis['five_hour_window_analysis']
        
        print("\n" + "="*80)
        print("DETAILED CONVERSATION TIMING ANALYSIS")
        print("="*80)
        
        if not stats:
            print("No conversation data found for analysis.")
            return
        
        # Overall conversation statistics
        durations = [c['duration_hours'] for c in stats]
        message_counts = [c['message_count'] for c in stats]
        
        print(f"\n📊 CONVERSATION OVERVIEW")
        print("-" * 50)
        print(f"Total conversations analyzed: {len(stats)}")
        print(f"Total messages: {sum(message_counts)}")
        print(f"Average conversation duration: {statistics.mean(durations):.2f} hours")
        print(f"Longest conversation: {max(durations):.2f} hours")
        print(f"Average messages per conversation: {statistics.mean(message_counts):.1f}")
        
        # Timing patterns
        if timing:
            print(f"\n⏱️  MESSAGE TIMING PATTERNS")
            print("-" * 50)
            print(f"Total message intervals: {timing['total_message_intervals']}")
            print(f"Average interval: {timing['avg_interval_minutes']:.2f} minutes")
            print(f"Median interval: {timing['median_interval_minutes']:.2f} minutes")
            print()
            print("Interval distribution:")
            for interval_range, count in timing['interval_distribution'].items():
                percentage = count / timing['total_message_intervals'] * 100
                print(f"  {interval_range:15} {count:6} intervals ({percentage:5.1f}%)")
        
        # Gap analysis
        if gaps:
            print(f"\n⏰ CONVERSATION GAP ANALYSIS")
            print("-" * 50)
            print(f"Conversations with 1+ hour gaps: {gaps['conversations_with_1h_gaps']}")
            print(f"Total 1+ hour gaps: {gaps['total_1h_gaps']}")
            print(f"Total 30+ minute gaps: {gaps['total_30min_gaps']}")
            if gaps['total_1h_gaps'] > 0:
                print(f"Average gap duration: {gaps['avg_gap_duration_hours']:.2f} hours")
                print(f"Maximum gap duration: {gaps['max_gap_duration_hours']:.2f} hours")
                print()
                print("Gap duration distribution:")
                for gap_range, count in gaps['gap_distribution'].items():
                    print(f"  {gap_range:15} {count:6} gaps")
        
        # Token flow analysis
        if tokens:
            print(f"\n🔥 TOKEN CONSUMPTION PATTERNS")
            print("-" * 50)
            print(f"Average token rate: {tokens['avg_tokens_per_hour']:.1f} tokens/hour")
            print(f"Median token rate: {tokens['median_tokens_per_hour']:.1f} tokens/hour")
            print(f"Peak token rate: {tokens['max_tokens_per_hour']:.1f} tokens/hour")
            print()
            print("Token rate distribution:")
            for rate_range, count in tokens['token_rate_distribution'].items():
                print(f"  {rate_range:15} {count:6} conversations")
        
        # Individual conversation details
        print(f"\n📋 INDIVIDUAL CONVERSATION ANALYSIS")
        print("-" * 50)
        sorted_stats = sorted(stats, key=lambda x: x['duration_hours'], reverse=True)
        
        for i, conv in enumerate(sorted_stats[:10], 1):  # Top 10 conversations
            print(f"\n{i}. Session: {conv['session_id'][:16]}...")
            print(f"   Duration: {conv['duration_hours']:.2f} hours | Messages: {conv['message_count']}")
            print(f"   Avg interval: {conv['avg_interval_minutes']:.1f}min | Max gap: {conv['max_gap_hours']:.1f}h")
            print(f"   Natural breaks (1+h gaps): {conv['natural_break_points']}")
            print(f"   Token rate: {conv['tokens_per_hour']:.1f} tokens/hour")
            
            if conv['bursts']:
                print(f"   Activity bursts: {len(conv['bursts'])}")
                for j, burst in enumerate(conv['bursts'][:3], 1):  # Show first 3 bursts
                    print(f"     Burst {j}: {burst['message_count']} msgs in {burst['duration_minutes']:.1f}min")
        
        # 5-hour window analysis
        if five_hour:
            print(f"\n🕐 5-HOUR WINDOW IMPLEMENTATION ANALYSIS")
            print("-" * 50)
            print(f"Conversations > 5 hours: {five_hour['conversations_over_5h']} ({five_hour['percentage_over_5h']:.1f}%)")
            print(f"Conversations with natural breaks: {five_hour['conversations_with_natural_breaks']} ({five_hour['percentage_with_natural_breaks']:.1f}%)")
            print(f"Window breaks needed: {five_hour['window_breaks_needed']}")
            print(f"Natural breaks available: {five_hour['natural_breaks_available']}")
            print(f"Break alignment ratio: {five_hour['break_alignment_ratio']:.2f}")
            print(f"Longest conversation: {five_hour['longest_conversation_hours']:.2f} hours")
            print(f"Avg natural breaks per conversation: {five_hour['avg_natural_breaks_per_conversation']:.2f}")
        
        # Recommendations
        print(f"\n🎯 IMPLEMENTATION RECOMMENDATIONS")
        print("-" * 50)
        
        if five_hour['percentage_over_5h'] < 5:
            print("✅ Very few conversations exceed 5 hours")
            print("✅ 5-hour windows should work well for most use cases")
        else:
            print("⚠️  Significant number of long conversations")
            print("⚠️  Consider conversation splitting strategies")
        
        if five_hour['break_alignment_ratio'] > 0.5:
            print("✅ Good alignment between natural breaks and needed window breaks")
            print("✅ Use 1+ hour gaps as natural split points")
        else:
            print("⚠️  Limited natural breaks for long conversations")
            print("⚠️  May need forced splits at 5-hour boundaries")
        
        if timing and timing['interval_distribution']['> 6 hours'] > 0:
            print("⚠️  Some very long gaps detected")
            print("⚠️  Consider these as definitive conversation boundaries")
        
        print("\n📝 SPECIFIC RECOMMENDATIONS:")
        print("1. Implement 5-hour sliding windows with conversation-aware boundaries")
        print("2. Use gaps >1 hour as preferred split points")
        print("3. For conversations >5h without natural breaks, implement gentle interruption")
        print("4. Track conversation state across window boundaries")
        if timing and timing['avg_interval_minutes'] < 2:
            print("5. High message frequency - ensure efficient real-time processing")

def main():
    analyzer = DetailedConversationAnalyzer()
    
    print("Starting detailed conversation timing analysis...")
    print("Analyzing message patterns, gaps, and 5-hour window implications...")
    print()
    
    analysis = analyzer.analyze_detailed_conversations()
    analyzer.display_detailed_analysis(analysis)

if __name__ == '__main__':
    main()