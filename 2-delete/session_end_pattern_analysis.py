#!/usr/bin/env python3
"""
Session End Pattern Analysis - Analyze conversation JSONL files for limit patterns
"""
import os
import json
import glob
from pathlib import Path
from datetime import datetime, timedelta
from collections import defaultdict, Counter
import statistics

class SessionEndAnalyzer:
    def __init__(self):
        self.home_dir = Path.home()
        
    def discover_claude_paths(self):
        """Discover all Claude installation paths"""
        paths = []
        claude_root = self.home_dir / '.claude'
        
        if claude_root.exists():
            paths.append(str(claude_root))
            
            # Check for VM instances
            vms_dir = claude_root / 'vms'
            if vms_dir.exists():
                for vm_dir in vms_dir.iterdir():
                    if vm_dir.is_dir():
                        vm_claude = vm_dir / '.claude'
                        if vm_claude.exists():
                            paths.append(str(vm_claude))
        
        return paths

    def analyze_conversation_endings(self):
        """Analyze conversation JSONL files for session ending patterns"""
        paths = self.discover_claude_paths()
        conversation_data = []
        
        for claude_path in paths:
            try:
                projects_dir = Path(claude_path) / 'projects'
                if not projects_dir.exists():
                    continue
                
                for session_dir in projects_dir.iterdir():
                    if not session_dir.is_dir():
                        continue
                    
                    # Find conversation JSONL files
                    jsonl_files = list(session_dir.glob('conversation_*.jsonl'))
                    
                    for jsonl_file in jsonl_files:
                        try:
                            session_analysis = self.analyze_single_conversation(jsonl_file, session_dir.name)
                            if session_analysis:
                                conversation_data.append(session_analysis)
                        except Exception as e:
                            print(f"Error analyzing {jsonl_file}: {e}")
                            continue
                            
            except Exception as e:
                print(f"Error processing {claude_path}: {e}")
                continue
        
        return conversation_data

    def analyze_single_conversation(self, jsonl_file, session_id):
        """Analyze a single conversation JSONL file for ending patterns"""
        entries = []
        
        try:
            with open(jsonl_file, 'r') as f:
                for line in f:
                    line = line.strip()
                    if line:
                        try:
                            entry = json.loads(line)
                            entries.append(entry)
                        except json.JSONDecodeError:
                            continue
        except Exception:
            return None
        
        if not entries:
            return None
        
        # Analyze the conversation
        analysis = {
            'session_id': session_id,
            'file_path': str(jsonl_file),
            'total_entries': len(entries),
            'first_timestamp': None,
            'last_timestamp': None,
            'duration_seconds': None,
            'total_input_tokens': 0,
            'total_output_tokens': 0,
            'total_tokens': 0,
            'final_entry_tokens': {},
            'abrupt_ending_indicators': [],
            'potential_limit_hit': False,
            'ending_characteristics': {},
            'token_progression': [],
            'conversation_flow': []
        }
        
        # Process entries chronologically
        for i, entry in enumerate(entries):
            # Extract timestamp
            timestamp = None
            for ts_field in ['timestamp', 'createdAt', 'time']:
                if ts_field in entry:
                    try:
                        timestamp = datetime.fromisoformat(str(entry[ts_field]).replace('Z', '+00:00'))
                        break
                    except:
                        continue
            
            if timestamp:
                if analysis['first_timestamp'] is None:
                    analysis['first_timestamp'] = timestamp
                analysis['last_timestamp'] = timestamp
            
            # Extract token information
            entry_tokens = {}
            for token_field in ['inputTokens', 'outputTokens', 'totalTokens', 'tokens']:
                if token_field in entry and isinstance(entry[token_field], (int, float)):
                    entry_tokens[token_field] = entry[token_field]
                    if token_field == 'inputTokens':
                        analysis['total_input_tokens'] += entry[token_field]
                    elif token_field == 'outputTokens':
                        analysis['total_output_tokens'] += entry[token_field]
                    elif token_field == 'totalTokens':
                        analysis['total_tokens'] = max(analysis['total_tokens'], entry[token_field])
            
            # Track token progression
            if entry_tokens:
                analysis['token_progression'].append({
                    'entry_index': i,
                    'timestamp': timestamp.isoformat() if timestamp else None,
                    'tokens': entry_tokens
                })
            
            # Track conversation flow
            flow_entry = {
                'entry_index': i,
                'type': entry.get('type', 'unknown'),
                'role': entry.get('role', 'unknown'),
                'has_content': bool(entry.get('content')),
                'tokens': entry_tokens
            }
            analysis['conversation_flow'].append(flow_entry)
            
            # If this is the last entry, capture its characteristics
            if i == len(entries) - 1:
                analysis['final_entry_tokens'] = entry_tokens
                analysis['ending_characteristics'] = {
                    'final_type': entry.get('type'),
                    'final_role': entry.get('role'),
                    'has_final_content': bool(entry.get('content')),
                    'final_model': entry.get('model'),
                    'final_cost': entry.get('costUSD'),
                    'has_error': 'error' in entry or 'errorType' in entry
                }
        
        # Calculate duration
        if analysis['first_timestamp'] and analysis['last_timestamp']:
            duration = (analysis['last_timestamp'] - analysis['first_timestamp']).total_seconds()
            analysis['duration_seconds'] = duration
        
        # Analyze for potential limit indicators
        self.detect_limit_patterns(analysis)
        
        return analysis

    def detect_limit_patterns(self, analysis):
        """Detect patterns that might indicate limit hits"""
        # Common Claude token limits
        common_limits = [200000, 150000, 100000, 75000, 50000, 25000, 10000, 8000, 4000]
        
        # Check if final token counts are near common limits
        for token_type, count in analysis['final_entry_tokens'].items():
            for limit in common_limits:
                if abs(count - limit) < (limit * 0.05):  # Within 5% of limit
                    analysis['potential_limit_hit'] = True
                    analysis['abrupt_ending_indicators'].append({
                        'type': 'near_token_limit',
                        'token_type': token_type,
                        'count': count,
                        'suspected_limit': limit,
                        'percentage': (count / limit) * 100
                    })
        
        # Check for abrupt endings (very short conversations with high token counts)
        if analysis['duration_seconds'] and analysis['duration_seconds'] < 300:  # < 5 minutes
            if analysis['total_tokens'] > 10000:  # But high token count
                analysis['abrupt_ending_indicators'].append({
                    'type': 'short_duration_high_tokens',
                    'duration': analysis['duration_seconds'],
                    'tokens': analysis['total_tokens']
                })
        
        # Check for sudden token jumps (might indicate limit approach)
        if len(analysis['token_progression']) > 1:
            for i in range(1, len(analysis['token_progression'])):
                prev_tokens = analysis['token_progression'][i-1]['tokens']
                curr_tokens = analysis['token_progression'][i]['tokens']
                
                for token_type in ['totalTokens', 'inputTokens', 'outputTokens']:
                    if token_type in prev_tokens and token_type in curr_tokens:
                        jump = curr_tokens[token_type] - prev_tokens[token_type]
                        if jump > 50000:  # Large token jump
                            analysis['abrupt_ending_indicators'].append({
                                'type': 'large_token_jump',
                                'token_type': token_type,
                                'jump_size': jump,
                                'from_entry': i-1,
                                'to_entry': i
                            })
        
        # Check for error indicators in ending
        if analysis['ending_characteristics'].get('has_error'):
            analysis['abrupt_ending_indicators'].append({
                'type': 'error_ending',
                'details': 'Final entry contains error information'
            })

    def generate_session_end_report(self):
        """Generate comprehensive session ending analysis report"""
        print("=" * 80)
        print("SESSION END PATTERN ANALYSIS REPORT")
        print("=" * 80)
        
        conversation_data = self.analyze_conversation_endings()
        
        if not conversation_data:
            print("\n❌ NO CONVERSATION DATA FOUND")
            return {'status': 'no_data'}
        
        print(f"\n📊 ANALYZED {len(conversation_data)} CONVERSATIONS")
        
        # Aggregate statistics
        stats = {
            'total_conversations': len(conversation_data),
            'with_limit_indicators': len([c for c in conversation_data if c['abrupt_ending_indicators']]),
            'potential_limit_hits': len([c for c in conversation_data if c['potential_limit_hit']]),
            'token_distributions': {
                'input': [],
                'output': [],
                'total': []
            },
            'duration_distributions': [],
            'ending_types': Counter(),
            'limit_patterns': Counter()
        }
        
        # Collect statistics
        for conv in conversation_data:
            if conv['total_input_tokens']:
                stats['token_distributions']['input'].append(conv['total_input_tokens'])
            if conv['total_output_tokens']:
                stats['token_distributions']['output'].append(conv['total_output_tokens'])
            if conv['total_tokens']:
                stats['token_distributions']['total'].append(conv['total_tokens'])
            
            if conv['duration_seconds']:
                stats['duration_distributions'].append(conv['duration_seconds'])
            
            ending_type = conv['ending_characteristics'].get('final_type', 'unknown')
            stats['ending_types'][ending_type] += 1
            
            for indicator in conv['abrupt_ending_indicators']:
                stats['limit_patterns'][indicator['type']] += 1
        
        # Display results
        print("\n" + "=" * 40)
        print("CONVERSATION STATISTICS")
        print("=" * 40)
        
        print(f"Total conversations: {stats['total_conversations']}")
        print(f"With limit indicators: {stats['with_limit_indicators']}")
        print(f"Potential limit hits: {stats['potential_limit_hits']}")
        
        print(f"\n📈 TOKEN DISTRIBUTIONS:")
        for token_type, values in stats['token_distributions'].items():
            if values:
                print(f"   {token_type.title()} tokens:")
                print(f"      Count: {len(values)}")
                print(f"      Range: {min(values):,} - {max(values):,}")
                print(f"      Average: {statistics.mean(values):,.1f}")
                print(f"      Median: {statistics.median(values):,.1f}")
        
        print(f"\n⏱️  DURATION ANALYSIS:")
        if stats['duration_distributions']:
            durations = stats['duration_distributions']
            print(f"   Sessions with timing: {len(durations)}")
            print(f"   Range: {min(durations):.1f}s - {max(durations):.1f}s")
            print(f"   Average: {statistics.mean(durations):.1f}s")
            print(f"   Median: {statistics.median(durations):.1f}s")
        
        print(f"\n🏁 ENDING TYPES:")
        for ending_type, count in stats['ending_types'].most_common():
            print(f"   {ending_type}: {count}")
        
        print(f"\n🚨 LIMIT PATTERNS:")
        if stats['limit_patterns']:
            for pattern_type, count in stats['limit_patterns'].most_common():
                print(f"   {pattern_type}: {count}")
        else:
            print("   No limit patterns detected")
        
        # Detailed limit analysis
        print("\n" + "=" * 40)
        print("DETAILED LIMIT ANALYSIS")
        print("=" * 40)
        
        limit_conversations = [c for c in conversation_data if c['potential_limit_hit']]
        
        if limit_conversations:
            print(f"\n🎯 {len(limit_conversations)} CONVERSATIONS WITH POTENTIAL LIMIT HITS:")
            
            for conv in limit_conversations:
                print(f"\n   Session: {conv['session_id']}")
                print(f"   Total tokens: {conv['total_tokens']:,}")
                print(f"   Duration: {conv['duration_seconds']:.1f}s" if conv['duration_seconds'] else "   Duration: Unknown")
                
                for indicator in conv['abrupt_ending_indicators']:
                    if indicator['type'] == 'near_token_limit':
                        print(f"   🚨 Near limit: {indicator['count']:,} tokens ({indicator['percentage']:.1f}% of {indicator['suspected_limit']:,})")
                    else:
                        print(f"   ⚠️  {indicator['type']}: {indicator}")
        else:
            print("\n✅ No clear limit hit patterns detected in conversation endings")
        
        # High token conversations
        high_token_convs = [c for c in conversation_data if c['total_tokens'] > 50000]
        if high_token_convs:
            print(f"\n📊 HIGH TOKEN CONVERSATIONS (>50k tokens):")
            high_token_convs.sort(key=lambda x: x['total_tokens'], reverse=True)
            
            for conv in high_token_convs[:10]:  # Top 10
                print(f"   {conv['session_id']}: {conv['total_tokens']:,} tokens")
                if conv['abrupt_ending_indicators']:
                    print(f"      ⚠️  Has limit indicators: {len(conv['abrupt_ending_indicators'])}")
        
        # Recommendations
        print("\n" + "=" * 40)
        print("LIMIT DETECTION STRATEGY")
        print("=" * 40)
        
        recommendations = [
            "📊 Monitor token counts in final conversation entries",
            "🎯 Set alerts for conversations approaching 95% of known limits",
            "⏱️  Track duration vs token count ratios for anomaly detection",
            "🔍 Look for patterns in conversation ending types",
            "📈 Implement real-time token tracking during conversations"
        ]
        
        # Specific recommendations based on findings
        if stats['potential_limit_hits'] > 0:
            recommendations.append(f"🚨 Found {stats['potential_limit_hits']} potential limit hits - implement immediate monitoring")
        
        if stats['limit_patterns']:
            most_common_pattern = stats['limit_patterns'].most_common(1)[0]
            recommendations.append(f"🔍 Focus on '{most_common_pattern[0]}' pattern (occurs {most_common_pattern[1]} times)")
        
        if high_token_convs:
            max_tokens = max(c['total_tokens'] for c in high_token_convs)
            recommendations.append(f"📏 Set warning threshold below {max_tokens:,} tokens based on observed maximum")
        
        print("\nRECOMMENDATIONS:")
        for i, rec in enumerate(recommendations, 1):
            print(f"{i}. {rec}")
        
        return {
            'status': 'success',
            'conversations_analyzed': len(conversation_data),
            'statistics': stats,
            'limit_conversations': limit_conversations,
            'high_token_conversations': high_token_convs,
            'recommendations': recommendations
        }

if __name__ == "__main__":
    analyzer = SessionEndAnalyzer()
    result = analyzer.generate_session_end_report()
    
    # Save detailed results
    with open('/workspace/session_end_analysis_results.json', 'w') as f:
        # Convert datetime objects to strings for JSON serialization
        def json_serializer(obj):
            if isinstance(obj, datetime):
                return obj.isoformat()
            return str(obj)
        
        json.dump(result, f, indent=2, default=json_serializer)
    
    print(f"\n💾 Detailed results saved to: /workspace/session_end_analysis_results.json")