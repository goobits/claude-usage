#!/usr/bin/env python3
"""
Conversation Duration Analysis - Deep dive into conversation patterns for 5-hour window optimization
"""
import os
import json
import glob
from pathlib import Path
from datetime import datetime, timedelta
from collections import defaultdict, Counter
import statistics

class ConversationAnalyzer:
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
    
    def extract_conversation_id(self, data):
        """Extract conversation ID from a data entry"""
        # Try different potential conversation ID fields
        if 'conversationId' in data:
            return data['conversationId']
        elif 'message' in data and 'conversationId' in data['message']:
            return data['message']['conversationId']
        elif 'conversation' in data and 'id' in data['conversation']:
            return data['conversation']['id']
        elif 'sessionId' in data:
            # Use sessionId as conversation identifier for Claude Code data
            return data['sessionId']
        return None
    
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
    
    def analyze_conversations(self):
        """Analyze all conversations for duration patterns"""
        paths = self.discover_claude_paths()
        print(f"🔍 Analyzing conversations across {len(paths)} Claude instances...")
        
        # Track conversations globally
        conversations = defaultdict(list)  # conversation_id -> list of (timestamp, session_dir, data)
        session_conversation_mapping = defaultdict(set)  # session_dir -> set of conversation_ids
        
        total_files = 0
        total_entries = 0
        
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
                                        
                                        # Only analyze entries with usage data (actual conversation messages)
                                        if not ('message' in data and 'usage' in data['message']):
                                            continue
                                        
                                        # Extract conversation ID and timestamp
                                        conv_id = self.extract_conversation_id(data)
                                        timestamp = None
                                        if 'timestamp' in data:
                                            timestamp = self.parse_timestamp(data['timestamp'])
                                        
                                        if conv_id and timestamp:
                                            conversations[conv_id].append((timestamp, session_dir.name, data))
                                            session_conversation_mapping[session_dir.name].add(conv_id)
                                            
                                    except json.JSONDecodeError:
                                        continue
                                        
                        except Exception as e:
                            print(f"❌ Error processing {jsonl_file}: {e}")
                            continue
                            
            except Exception as e:
                print(f"❌ Error processing {claude_path}: {e}")
                continue
        
        print(f"📁 Processed {total_files} files with {total_entries} total entries")
        print(f"🗨️  Found {len(conversations)} unique conversations")
        print()
        
        # Analyze conversation patterns
        return self.analyze_conversation_patterns(conversations, session_conversation_mapping)
    
    def analyze_conversation_patterns(self, conversations, session_conversation_mapping):
        """Deep analysis of conversation duration and patterns"""
        # Calculate conversation durations and patterns
        conversation_stats = []
        long_conversations = []  # >5 hours
        conversation_gaps = []  # gaps within conversations
        session_boundaries_analysis = []
        
        for conv_id, entries in conversations.items():
            if len(entries) < 2:
                continue  # Skip single-message conversations
            
            # Sort entries by timestamp
            entries.sort(key=lambda x: x[0])
            
            # Calculate conversation duration
            start_time = entries[0][0]
            end_time = entries[-1][0]
            duration_hours = (end_time - start_time).total_seconds() / 3600
            
            # Analyze message frequency and gaps
            message_intervals = []
            large_gaps = []  # gaps > 1 hour
            sessions_involved = set(entry[1] for entry in entries)
            
            for i in range(1, len(entries)):
                prev_time = entries[i-1][0]
                curr_time = entries[i][0]
                interval_minutes = (curr_time - prev_time).total_seconds() / 60
                message_intervals.append(interval_minutes)
                
                if interval_minutes > 60:  # Gap > 1 hour
                    large_gaps.append({
                        'gap_hours': interval_minutes / 60,
                        'before_session': entries[i-1][1],
                        'after_session': entries[i][1],
                        'before_time': prev_time,
                        'after_time': curr_time
                    })
            
            # Calculate message rate (messages per hour)
            message_rate = len(entries) / max(duration_hours, 0.017)  # min 1 minute
            
            conv_stat = {
                'conversation_id': conv_id,
                'duration_hours': duration_hours,
                'message_count': len(entries),
                'message_rate_per_hour': message_rate,
                'sessions_involved': len(sessions_involved),
                'session_names': list(sessions_involved),
                'start_time': start_time,
                'end_time': end_time,
                'large_gaps': large_gaps,
                'message_intervals': message_intervals,
                'crosses_session_boundary': len(sessions_involved) > 1
            }
            
            conversation_stats.append(conv_stat)
            
            # Track long conversations (>5 hours)
            if duration_hours > 5:
                long_conversations.append(conv_stat)
            
            # Track conversations with large gaps
            if large_gaps:
                conversation_gaps.extend(large_gaps)
            
            # Analyze session boundary alignment
            if len(sessions_involved) > 1:
                session_boundaries_analysis.append({
                    'conversation_id': conv_id,
                    'duration_hours': duration_hours,
                    'sessions': list(sessions_involved),
                    'message_count': len(entries),
                    'crosses_boundaries': True
                })
        
        # Sort conversations by duration
        conversation_stats.sort(key=lambda x: x['duration_hours'], reverse=True)
        
        return {
            'conversation_stats': conversation_stats,
            'long_conversations': long_conversations,
            'conversation_gaps': conversation_gaps,
            'session_boundaries_analysis': session_boundaries_analysis,
            'session_conversation_mapping': dict(session_conversation_mapping)
        }
    
    def display_analysis(self, analysis):
        """Display comprehensive conversation analysis"""
        stats = analysis['conversation_stats']
        long_convs = analysis['long_conversations']
        gaps = analysis['conversation_gaps']
        boundaries = analysis['session_boundaries_analysis']
        
        print("="*80)
        print("CONVERSATION DURATION ANALYSIS")
        print("="*80)
        print()
        
        # Overall statistics
        if stats:
            durations = [c['duration_hours'] for c in stats]
            message_counts = [c['message_count'] for c in stats]
            message_rates = [c['message_rate_per_hour'] for c in stats]
            
            print("📊 OVERALL STATISTICS")
            print("-" * 40)
            print(f"Total conversations: {len(stats)}")
            print(f"Average duration: {statistics.mean(durations):.2f} hours")
            print(f"Median duration: {statistics.median(durations):.2f} hours")
            print(f"Max duration: {max(durations):.2f} hours")
            print(f"Min duration: {min(durations):.2f} hours")
            print(f"Conversations > 5 hours: {len(long_convs)} ({len(long_convs)/len(stats)*100:.1f}%)")
            print(f"Conversations crossing session boundaries: {len(boundaries)} ({len(boundaries)/len(stats)*100:.1f}%)")
            print()
            
            # Duration distribution
            print("📈 DURATION DISTRIBUTION")
            print("-" * 40)
            duration_buckets = {
                "< 1 hour": len([d for d in durations if d < 1]),
                "1-2 hours": len([d for d in durations if 1 <= d < 2]),
                "2-5 hours": len([d for d in durations if 2 <= d < 5]),
                "5-10 hours": len([d for d in durations if 5 <= d < 10]),
                "10+ hours": len([d for d in durations if d >= 10])
            }
            
            for bucket, count in duration_buckets.items():
                percentage = count / len(stats) * 100
                print(f"{bucket:12} {count:6} conversations ({percentage:5.1f}%)")
            print()
        
        # Top 10 longest conversations
        print("🏆 TOP 10 LONGEST CONVERSATIONS")
        print("-" * 40)
        for i, conv in enumerate(stats[:10], 1):
            sessions_info = f"{conv['sessions_involved']} sessions" if conv['sessions_involved'] > 1 else "1 session"
            gaps_info = f", {len(conv['large_gaps'])} gaps >1h" if conv['large_gaps'] else ""
            print(f"{i:2}. {conv['duration_hours']:6.2f}h | {conv['message_count']:4} msgs | {sessions_info}{gaps_info}")
            if conv['large_gaps']:
                for gap in conv['large_gaps'][:3]:  # Show first 3 gaps
                    print(f"    └─ Gap: {gap['gap_hours']:.1f}h between {gap['before_session']} and {gap['after_session']}")
        print()
        
        # Conversations > 5 hours detailed analysis
        if long_convs:
            print("🔥 CONVERSATIONS > 5 HOURS (DETAILED)")
            print("-" * 40)
            for conv in long_convs:
                print(f"Conversation: {conv['conversation_id'][:16]}...")
                print(f"  Duration: {conv['duration_hours']:.2f} hours")
                print(f"  Messages: {conv['message_count']}")
                print(f"  Rate: {conv['message_rate_per_hour']:.1f} messages/hour")
                print(f"  Sessions: {conv['sessions_involved']} ({', '.join(conv['session_names'][:3])}{'...' if len(conv['session_names']) > 3 else ''})")
                print(f"  Large gaps (>1h): {len(conv['large_gaps'])}")
                
                if conv['large_gaps']:
                    print("  Gap details:")
                    for gap in conv['large_gaps'][:5]:  # Show first 5 gaps
                        print(f"    - {gap['gap_hours']:.1f}h gap at {gap['after_time'].strftime('%Y-%m-%d %H:%M')}")
                
                # Message frequency analysis within conversation
                intervals = conv['message_intervals']
                if intervals:
                    avg_interval = statistics.mean(intervals)
                    median_interval = statistics.median(intervals)
                    max_interval = max(intervals)
                    print(f"  Message intervals: avg={avg_interval:.1f}min, median={median_interval:.1f}min, max={max_interval:.1f}min")
                print()
        
        # Large gaps analysis
        if gaps:
            print("⏰ LARGE GAPS WITHIN CONVERSATIONS (>1 HOUR)")
            print("-" * 40)
            gap_hours = [g['gap_hours'] for g in gaps]
            print(f"Total large gaps: {len(gaps)}")
            print(f"Average gap: {statistics.mean(gap_hours):.2f} hours")
            print(f"Median gap: {statistics.median(gap_hours):.2f} hours")
            print(f"Largest gap: {max(gap_hours):.2f} hours")
            print()
            
            # Show largest gaps
            sorted_gaps = sorted(gaps, key=lambda x: x['gap_hours'], reverse=True)
            print("Largest gaps:")
            for gap in sorted_gaps[:10]:
                cross_session = "cross-session" if gap['before_session'] != gap['after_session'] else "same session"
                print(f"  {gap['gap_hours']:6.1f}h | {gap['after_time'].strftime('%Y-%m-%d %H:%M')} | {cross_session}")
            print()
        
        # Session boundary analysis
        if boundaries:
            print("🔀 CONVERSATIONS CROSSING SESSION BOUNDARIES")
            print("-" * 40)
            cross_session_durations = [b['duration_hours'] for b in boundaries]
            print(f"Conversations crossing boundaries: {len(boundaries)}")
            print(f"Average duration (cross-boundary): {statistics.mean(cross_session_durations):.2f} hours")
            print(f"Longest cross-boundary conversation: {max(cross_session_durations):.2f} hours")
            print()
            
            # Show examples
            print("Examples of cross-boundary conversations:")
            sorted_boundaries = sorted(boundaries, key=lambda x: x['duration_hours'], reverse=True)
            for boundary in sorted_boundaries[:10]:
                print(f"  {boundary['duration_hours']:6.2f}h | {boundary['message_count']:4} msgs | {len(boundary['sessions'])} sessions")
            print()
        
        # Message rate analysis
        if stats:
            rates = [c['message_rate_per_hour'] for c in stats]
            print("💬 MESSAGE RATE ANALYSIS")
            print("-" * 40)
            print(f"Average message rate: {statistics.mean(rates):.1f} messages/hour")
            print(f"Median message rate: {statistics.median(rates):.1f} messages/hour")
            print(f"Fastest conversation: {max(rates):.1f} messages/hour")
            print(f"Slowest conversation: {min(rates):.1f} messages/hour")
            print()
            
            # Rate distribution
            rate_buckets = {
                "< 1 msg/hour": len([r for r in rates if r < 1]),
                "1-5 msg/hour": len([r for r in rates if 1 <= r < 5]),
                "5-10 msg/hour": len([r for r in rates if 5 <= r < 10]),
                "10-20 msg/hour": len([r for r in rates if 10 <= r < 20]),
                "20+ msg/hour": len([r for r in rates if r >= 20])
            }
            
            print("Message rate distribution:")
            for bucket, count in rate_buckets.items():
                percentage = count / len(stats) * 100
                print(f"  {bucket:15} {count:6} conversations ({percentage:5.1f}%)")
            print()
        
        # Summary and recommendations
        print("🎯 SUMMARY & RECOMMENDATIONS FOR 5-HOUR WINDOWS")
        print("-" * 40)
        
        long_conv_percentage = len(long_convs) / len(stats) * 100 if stats else 0
        cross_boundary_percentage = len(boundaries) / len(stats) * 100 if stats else 0
        
        print(f"1. {long_conv_percentage:.1f}% of conversations exceed 5 hours")
        if long_conv_percentage > 5:
            print("   ⚠️  Significant number of long conversations - need special handling")
        else:
            print("   ✅ Few long conversations - 5-hour windows should work well")
        
        print(f"2. {cross_boundary_percentage:.1f}% of conversations cross session boundaries")
        if cross_boundary_percentage > 10:
            print("   ⚠️  Many conversations span multiple sessions")
        else:
            print("   ✅ Most conversations stay within session boundaries")
        
        large_gap_conversations = len(set(gap.get('conversation_id', 'unknown') for gap in gaps))
        if gaps:
            print(f"3. {large_gap_conversations} conversations have gaps >1 hour")
            avg_gap = statistics.mean([g['gap_hours'] for g in gaps])
            print(f"   Average large gap: {avg_gap:.1f} hours")
            if avg_gap > 3:
                print("   ⚠️  Large gaps suggest natural session breaks within conversations")
            else:
                print("   ✅ Gaps are manageable for 5-hour windows")
        
        print()
        print("📋 IMPLEMENTATION RECOMMENDATIONS:")
        if long_conv_percentage < 5 and cross_boundary_percentage < 10:
            print("✅ 5-hour windows should work well with current conversation patterns")
            print("✅ Most conversations naturally align with session boundaries")
        else:
            print("⚠️  Consider these optimizations:")
            if long_conv_percentage > 5:
                print("   - Implement conversation splitting for >5 hour conversations")
                print("   - Use natural gaps (>1 hour) as split points")
            if cross_boundary_percentage > 10:
                print("   - Track conversations across session boundaries")
                print("   - Consider conversation-based rather than session-based windows")

def main():
    analyzer = ConversationAnalyzer()
    
    print("Starting comprehensive conversation duration analysis...")
    print("This may take a few minutes for large datasets...")
    print()
    
    analysis = analyzer.analyze_conversations()
    analyzer.display_analysis(analysis)

if __name__ == '__main__':
    main()