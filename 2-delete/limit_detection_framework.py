#!/usr/bin/env python3
"""
Comprehensive Limit Detection Framework for Claude Usage Analysis
"""
import json
import statistics
from datetime import datetime, timedelta
from collections import Counter, defaultdict
from typing import Dict, List, Optional, Tuple

class LimitDetectionFramework:
    """
    Comprehensive framework for detecting Claude usage limits across conversations and sessions
    """
    
    # Known Claude model limits (tokens)
    CLAUDE_LIMITS = {
        'claude-3-5-sonnet-20241022': 200000,
        'claude-3-5-sonnet-20240620': 200000,
        'claude-3-5-haiku-20241022': 200000,
        'claude-3-opus-20240229': 200000,
        'claude-3-sonnet-20240229': 200000,
        'claude-3-haiku-20240307': 200000,
        'claude-2.1': 200000,
        'claude-2.0': 100000,
        'claude-instant-1.2': 100000,
        # Default fallback
        'default': 200000
    }
    
    def __init__(self, warning_threshold_percentage: float = 0.9):
        """
        Initialize the limit detection framework
        
        Args:
            warning_threshold_percentage: Percentage of limit to trigger warnings (default 90%)
        """
        self.warning_threshold = warning_threshold_percentage
        self.detection_methods = {
            'token_threshold': self._detect_token_threshold,
            'conversation_abrupt_end': self._detect_abrupt_endings,
            'session_block_analysis': self._detect_session_block_limits,
            'cost_pattern_analysis': self._detect_cost_patterns,
            'time_based_analysis': self._detect_time_patterns
        }
    
    def detect_limits_in_data(self, conversations: List[Dict], session_blocks: List[Dict] = None) -> Dict:
        """
        Comprehensive limit detection across all available data
        
        Args:
            conversations: List of conversation data from JSONL files
            session_blocks: Optional list of session block data
            
        Returns:
            Dict containing limit detection results and recommendations
        """
        results = {
            'summary': {
                'total_conversations': len(conversations),
                'limit_hits_detected': 0,
                'warnings_generated': 0,
                'risk_level': 'low'
            },
            'detection_results': {},
            'limit_events': [],
            'recommendations': [],
            'monitoring_config': {}
        }
        
        # Run all detection methods
        for method_name, method_func in self.detection_methods.items():
            try:
                if method_name == 'session_block_analysis' and session_blocks:
                    method_result = method_func(session_blocks)
                else:
                    method_result = method_func(conversations)
                
                results['detection_results'][method_name] = method_result
                
                # Aggregate findings
                if method_result.get('limit_hits'):
                    results['summary']['limit_hits_detected'] += len(method_result['limit_hits'])
                    results['limit_events'].extend(method_result['limit_hits'])
                
                if method_result.get('warnings'):
                    results['summary']['warnings_generated'] += len(method_result['warnings'])
                    
            except Exception as e:
                results['detection_results'][method_name] = {'error': str(e)}
        
        # Determine overall risk level
        results['summary']['risk_level'] = self._calculate_risk_level(results)
        
        # Generate recommendations
        results['recommendations'] = self._generate_recommendations(results)
        
        # Create monitoring configuration
        results['monitoring_config'] = self._generate_monitoring_config(results)
        
        return results
    
    def _detect_token_threshold(self, conversations: List[Dict]) -> Dict:
        """Detect limit hits based on token thresholds"""
        results = {
            'method': 'token_threshold',
            'limit_hits': [],
            'warnings': [],
            'statistics': {},
            'patterns': {}
        }
        
        token_counts = []
        model_usage = defaultdict(list)
        
        for conv in conversations:
            # Extract token information
            total_tokens = conv.get('total_tokens', 0)
            model = conv.get('model', 'unknown')
            
            if total_tokens > 0:
                token_counts.append(total_tokens)
                model_usage[model].append(total_tokens)
                
                # Get model limit
                limit = self.CLAUDE_LIMITS.get(model, self.CLAUDE_LIMITS['default'])
                warning_threshold = limit * self.warning_threshold
                
                # Check for limit hits (within 5% of limit)
                if total_tokens >= limit * 0.95:
                    results['limit_hits'].append({
                        'session_id': conv.get('session_id'),
                        'tokens': total_tokens,
                        'limit': limit,
                        'percentage': (total_tokens / limit) * 100,
                        'model': model,
                        'timestamp': conv.get('last_timestamp')
                    })
                
                # Check for warnings
                elif total_tokens >= warning_threshold:
                    results['warnings'].append({
                        'session_id': conv.get('session_id'),
                        'tokens': total_tokens,
                        'limit': limit,
                        'percentage': (total_tokens / limit) * 100,
                        'model': model,
                        'warning_level': 'approaching_limit'
                    })
        
        # Calculate statistics
        if token_counts:
            results['statistics'] = {
                'total_conversations': len(token_counts),
                'min_tokens': min(token_counts),
                'max_tokens': max(token_counts),
                'avg_tokens': statistics.mean(token_counts),
                'median_tokens': statistics.median(token_counts),
                'std_dev': statistics.stdev(token_counts) if len(token_counts) > 1 else 0
            }
        
        # Analyze patterns by model
        for model, tokens in model_usage.items():
            if tokens:
                limit = self.CLAUDE_LIMITS.get(model, self.CLAUDE_LIMITS['default'])
                results['patterns'][model] = {
                    'usage_count': len(tokens),
                    'max_tokens': max(tokens),
                    'avg_percentage_of_limit': (statistics.mean(tokens) / limit) * 100,
                    'max_percentage_of_limit': (max(tokens) / limit) * 100
                }
        
        return results
    
    def _detect_abrupt_endings(self, conversations: List[Dict]) -> Dict:
        """Detect conversations that end abruptly, potentially due to limits"""
        results = {
            'method': 'abrupt_endings',
            'limit_hits': [],
            'warnings': [],
            'patterns': {}
        }
        
        abrupt_indicators = []
        
        for conv in conversations:
            indicators = []
            
            # Short duration with high tokens
            if conv.get('duration_seconds') and conv.get('total_tokens'):
                duration = conv['duration_seconds']
                tokens = conv['total_tokens']
                
                if duration < 300 and tokens > 50000:  # < 5 min but >50k tokens
                    indicators.append('short_duration_high_tokens')
                
                # High token velocity (tokens per minute)
                if duration > 0:
                    velocity = tokens / (duration / 60)  # tokens per minute
                    if velocity > 5000:  # Very high velocity
                        indicators.append('high_token_velocity')
            
            # Check conversation flow patterns
            flow = conv.get('conversation_flow', [])
            if flow:
                # Sudden stop after user message (no assistant response)
                if len(flow) > 1 and flow[-1].get('role') == 'user' and flow[-2].get('role') == 'user':
                    indicators.append('no_assistant_response')
                
                # Large token jumps between messages
                token_progression = conv.get('token_progression', [])
                if len(token_progression) > 1:
                    for i in range(1, len(token_progression)):
                        prev_tokens = token_progression[i-1].get('tokens', {}).get('totalTokens', 0)
                        curr_tokens = token_progression[i].get('tokens', {}).get('totalTokens', 0)
                        
                        if curr_tokens - prev_tokens > 50000:  # Large jump
                            indicators.append('large_token_jump')
                            break
            
            # Error in final entry
            if conv.get('ending_characteristics', {}).get('has_error'):
                indicators.append('error_ending')
            
            if indicators:
                entry = {
                    'session_id': conv.get('session_id'),
                    'indicators': indicators,
                    'tokens': conv.get('total_tokens', 0),
                    'duration': conv.get('duration_seconds'),
                    'severity': len(indicators)
                }
                
                abrupt_indicators.append(entry)
                
                # Classify as limit hit or warning based on severity
                if len(indicators) >= 2 or 'large_token_jump' in indicators:
                    results['limit_hits'].append(entry)
                else:
                    results['warnings'].append(entry)
        
        results['patterns'] = {
            'total_abrupt_endings': len(abrupt_indicators),
            'indicator_frequency': Counter([ind for entry in abrupt_indicators for ind in entry['indicators']])
        }
        
        return results
    
    def _detect_session_block_limits(self, session_blocks: List[Dict]) -> Dict:
        """Detect limits in session block data"""
        results = {
            'method': 'session_blocks',
            'limit_hits': [],
            'warnings': [],
            'patterns': {}
        }
        
        # This would analyze session block specific fields
        # For demonstration, looking for common patterns
        
        for block in session_blocks:
            # Look for explicit limit fields
            limit_fields = ['limitReached', 'limitHit', 'quotaExceeded', 'maxTokensReached']
            for field in limit_fields:
                if block.get(field):
                    results['limit_hits'].append({
                        'session_id': block.get('sessionId'),
                        'limit_type': field,
                        'timestamp': block.get('endTime'),
                        'tokens': block.get('totalTokens')
                    })
            
            # Check session end reasons
            end_reason = block.get('endReason', '').lower()
            if any(term in end_reason for term in ['limit', 'quota', 'exceeded', 'maximum']):
                results['limit_hits'].append({
                    'session_id': block.get('sessionId'),
                    'end_reason': end_reason,
                    'timestamp': block.get('endTime')
                })
        
        return results
    
    def _detect_cost_patterns(self, conversations: List[Dict]) -> Dict:
        """Detect unusual cost patterns that might indicate limits"""
        results = {
            'method': 'cost_patterns',
            'limit_hits': [],
            'warnings': [],
            'patterns': {}
        }
        
        # Analyze cost patterns - sudden spikes or stops
        costs = []
        for conv in conversations:
            total_cost = conv.get('total_cost', 0)
            if total_cost > 0:
                costs.append({
                    'session_id': conv.get('session_id'),
                    'cost': total_cost,
                    'tokens': conv.get('total_tokens', 0),
                    'timestamp': conv.get('last_timestamp')
                })
        
        if costs:
            # Sort by timestamp to analyze progression
            costs.sort(key=lambda x: x['timestamp'] if x['timestamp'] else '')
            
            # Look for sudden cost spikes (might indicate rapid token consumption before limit)
            if len(costs) > 1:
                cost_values = [c['cost'] for c in costs]
                avg_cost = statistics.mean(cost_values)
                std_cost = statistics.stdev(cost_values) if len(cost_values) > 1 else 0
                
                for cost_entry in costs:
                    if cost_entry['cost'] > avg_cost + (2 * std_cost):  # 2 standard deviations above mean
                        results['warnings'].append({
                            'session_id': cost_entry['session_id'],
                            'cost': cost_entry['cost'],
                            'avg_cost': avg_cost,
                            'warning_type': 'cost_spike'
                        })
            
            results['patterns'] = {
                'total_sessions_with_cost': len(costs),
                'avg_cost': statistics.mean([c['cost'] for c in costs]),
                'max_cost': max([c['cost'] for c in costs]),
                'cost_distribution': {
                    'low': len([c for c in costs if c['cost'] < 1.0]),
                    'medium': len([c for c in costs if 1.0 <= c['cost'] < 10.0]),
                    'high': len([c for c in costs if c['cost'] >= 10.0])
                }
            }
        
        return results
    
    def _detect_time_patterns(self, conversations: List[Dict]) -> Dict:
        """Detect time-based patterns that might indicate limits"""
        results = {
            'method': 'time_patterns',
            'limit_hits': [],
            'warnings': [],
            'patterns': {}
        }
        
        # Analyze time patterns - sessions that end at specific times might indicate daily/hourly limits
        session_times = []
        for conv in conversations:
            if conv.get('last_timestamp'):
                try:
                    timestamp = datetime.fromisoformat(conv['last_timestamp'].replace('Z', '+00:00'))
                    session_times.append({
                        'session_id': conv.get('session_id'),
                        'end_time': timestamp,
                        'hour': timestamp.hour,
                        'tokens': conv.get('total_tokens', 0)
                    })
                except:
                    continue
        
        if session_times:
            # Check for clustering at specific hours (might indicate daily resets)
            hour_distribution = Counter([s['hour'] for s in session_times])
            
            # Check for unusually high activity at specific hours
            avg_sessions_per_hour = len(session_times) / 24
            for hour, count in hour_distribution.items():
                if count > avg_sessions_per_hour * 2:  # More than 2x average
                    results['patterns'][f'high_activity_hour_{hour}'] = {
                        'sessions': count,
                        'average': avg_sessions_per_hour
                    }
        
        return results
    
    def _calculate_risk_level(self, results: Dict) -> str:
        """Calculate overall risk level based on detection results"""
        total_limit_hits = results['summary']['limit_hits_detected']
        total_warnings = results['summary']['warnings_generated']
        total_conversations = results['summary']['total_conversations']
        
        if total_conversations == 0:
            return 'unknown'
        
        limit_hit_rate = total_limit_hits / total_conversations
        warning_rate = total_warnings / total_conversations
        
        if limit_hit_rate > 0.1:  # More than 10% hit limits
            return 'critical'
        elif limit_hit_rate > 0.05 or warning_rate > 0.2:  # 5% hit limits or 20% warnings
            return 'high'
        elif limit_hit_rate > 0 or warning_rate > 0.1:  # Any limits or 10% warnings
            return 'medium'
        else:
            return 'low'
    
    def _generate_recommendations(self, results: Dict) -> List[str]:
        """Generate actionable recommendations based on detection results"""
        recommendations = []
        risk_level = results['summary']['risk_level']
        
        # Risk-based recommendations
        if risk_level == 'critical':
            recommendations.extend([
                "🚨 CRITICAL: Implement immediate token monitoring and alerts",
                "🚨 Set up automatic conversation checkpointing before limits",
                "🚨 Consider upgrading to higher-limit models or splitting workflows"
            ])
        elif risk_level == 'high':
            recommendations.extend([
                "⚠️  HIGH RISK: Implement proactive limit monitoring",
                "⚠️  Set warning alerts at 80% and 90% of token limits"
            ])
        elif risk_level == 'medium':
            recommendations.extend([
                "⚠️  Monitor token usage trends and set up basic alerts",
                "⚠️  Track conversations approaching 75% of limits"
            ])
        
        # Method-specific recommendations
        token_results = results['detection_results'].get('token_threshold', {})
        if token_results.get('limit_hits'):
            recommendations.append("📊 Implement real-time token counting during conversations")
            recommendations.append("📏 Set model-specific warning thresholds")
        
        abrupt_results = results['detection_results'].get('conversation_abrupt_end', {})
        if abrupt_results.get('patterns', {}).get('total_abrupt_endings', 0) > 0:
            recommendations.append("🔍 Monitor conversation flow patterns for abrupt endings")
            recommendations.append("⏱️  Track token velocity (tokens/minute) as early warning")
        
        # General recommendations
        recommendations.extend([
            "📈 Implement daily usage reports with limit proximity warnings",
            "🔄 Set up automated session rotation before approaching limits",
            "💾 Create conversation state backup before high-token operations",
            "📱 Add user-facing token usage indicators in the interface"
        ])
        
        return recommendations
    
    def _generate_monitoring_config(self, results: Dict) -> Dict:
        """Generate monitoring configuration based on analysis"""
        config = {
            'token_thresholds': {},
            'alert_levels': {},
            'monitoring_frequency': 'real_time',
            'metrics_to_track': []
        }
        
        # Set model-specific thresholds
        for model, limit in self.CLAUDE_LIMITS.items():
            if model != 'default':
                config['token_thresholds'][model] = {
                    'warning_threshold': int(limit * 0.8),
                    'critical_threshold': int(limit * 0.9),
                    'limit': limit
                }
        
        # Configure alert levels based on risk
        risk_level = results['summary']['risk_level']
        if risk_level in ['critical', 'high']:
            config['alert_levels'] = {
                'token_warning': 0.75,  # 75% of limit
                'token_critical': 0.9,   # 90% of limit
                'velocity_warning': 3000,  # tokens per minute
                'duration_anomaly': 600   # very short sessions in seconds
            }
        else:
            config['alert_levels'] = {
                'token_warning': 0.8,   # 80% of limit
                'token_critical': 0.95,  # 95% of limit
                'velocity_warning': 5000,
                'duration_anomaly': 300
            }
        
        # Essential metrics to track
        config['metrics_to_track'] = [
            'real_time_token_count',
            'session_duration',
            'tokens_per_minute',
            'conversation_flow_patterns',
            'model_usage_distribution',
            'cost_accumulation',
            'limit_approach_frequency'
        ]
        
        return config

def create_demo_analysis():
    """Create a demonstration of the limit detection framework with simulated data"""
    print("=" * 80)
    print("LIMIT DETECTION FRAMEWORK DEMONSTRATION")
    print("=" * 80)
    
    # Create simulated conversation data
    simulated_conversations = [
        {
            'session_id': 'conv_001',
            'total_tokens': 195000,  # Near limit
            'model': 'claude-3-5-sonnet-20241022',
            'duration_seconds': 3600,
            'total_cost': 15.60,
            'last_timestamp': '2024-01-15T14:30:00Z',
            'ending_characteristics': {'has_error': False},
            'conversation_flow': [
                {'role': 'user', 'tokens': {'totalTokens': 50000}},
                {'role': 'assistant', 'tokens': {'totalTokens': 100000}},
                {'role': 'user', 'tokens': {'totalTokens': 150000}},
                {'role': 'assistant', 'tokens': {'totalTokens': 195000}}
            ],
            'token_progression': [
                {'entry_index': 0, 'tokens': {'totalTokens': 50000}},
                {'entry_index': 1, 'tokens': {'totalTokens': 100000}},
                {'entry_index': 2, 'tokens': {'totalTokens': 150000}},
                {'entry_index': 3, 'tokens': {'totalTokens': 195000}}
            ]
        },
        {
            'session_id': 'conv_002',
            'total_tokens': 85000,  # Medium usage
            'model': 'claude-3-5-sonnet-20241022',
            'duration_seconds': 1800,
            'total_cost': 6.80,
            'last_timestamp': '2024-01-15T15:45:00Z',
            'ending_characteristics': {'has_error': False},
            'conversation_flow': [
                {'role': 'user', 'tokens': {'totalTokens': 20000}},
                {'role': 'assistant', 'tokens': {'totalTokens': 50000}},
                {'role': 'user', 'tokens': {'totalTokens': 70000}},
                {'role': 'assistant', 'tokens': {'totalTokens': 85000}}
            ]
        },
        {
            'session_id': 'conv_003',
            'total_tokens': 199500,  # Very close to limit
            'model': 'claude-3-5-sonnet-20241022',
            'duration_seconds': 120,  # Very short duration - suspicious
            'total_cost': 19.95,
            'last_timestamp': '2024-01-15T16:20:00Z',
            'ending_characteristics': {'has_error': True},
            'conversation_flow': [
                {'role': 'user', 'tokens': {'totalTokens': 100000}},
                {'role': 'assistant', 'tokens': {'totalTokens': 199500}}
            ],
            'token_progression': [
                {'entry_index': 0, 'tokens': {'totalTokens': 100000}},
                {'entry_index': 1, 'tokens': {'totalTokens': 199500}}  # Large jump
            ]
        },
        {
            'session_id': 'conv_004',
            'total_tokens': 25000,   # Low usage
            'model': 'claude-3-haiku-20240307',
            'duration_seconds': 900,
            'total_cost': 1.25,
            'last_timestamp': '2024-01-15T17:10:00Z',
            'ending_characteristics': {'has_error': False}
        }
    ]
    
    # Create simulated session blocks (optional)
    simulated_session_blocks = [
        {
            'sessionId': 'conv_003',
            'endTime': '2024-01-15T16:20:00Z',
            'endReason': 'token_limit_exceeded',
            'totalTokens': 199500,
            'limitReached': True
        }
    ]
    
    # Initialize and run framework
    framework = LimitDetectionFramework(warning_threshold_percentage=0.9)
    results = framework.detect_limits_in_data(simulated_conversations, simulated_session_blocks)
    
    # Display results
    print(f"\n📊 ANALYSIS SUMMARY:")
    print(f"   Total conversations analyzed: {results['summary']['total_conversations']}")
    print(f"   Limit hits detected: {results['summary']['limit_hits_detected']}")
    print(f"   Warnings generated: {results['summary']['warnings_generated']}")
    print(f"   Overall risk level: {results['summary']['risk_level'].upper()}")
    
    print(f"\n🔍 DETECTION METHOD RESULTS:")
    for method, result in results['detection_results'].items():
        print(f"\n   {method.replace('_', ' ').title()}:")
        if 'error' in result:
            print(f"      Error: {result['error']}")
        else:
            print(f"      Limit hits: {len(result.get('limit_hits', []))}")
            print(f"      Warnings: {len(result.get('warnings', []))}")
            
            # Show specific findings
            if result.get('limit_hits'):
                print(f"      Limit hit details:")
                for hit in result['limit_hits']:
                    if 'tokens' in hit:
                        print(f"         Session {hit.get('session_id')}: {hit.get('tokens', 0):,} tokens ({hit.get('percentage', 0):.1f}% of limit)")
                    else:
                        print(f"         Session {hit.get('session_id')}: {hit}")
    
    print(f"\n💡 RECOMMENDATIONS:")
    for i, rec in enumerate(results['recommendations'], 1):
        print(f"   {i}. {rec}")
    
    print(f"\n⚙️  MONITORING CONFIGURATION:")
    config = results['monitoring_config']
    print(f"   Alert frequency: {config['monitoring_frequency']}")
    print(f"   Key thresholds:")
    for level, threshold in config['alert_levels'].items():
        if isinstance(threshold, float) and threshold < 1:
            print(f"      {level}: {threshold*100:.0f}% of limit")
        else:
            print(f"      {level}: {threshold}")
    
    print(f"\n   Metrics to track: {len(config['metrics_to_track'])}")
    for metric in config['metrics_to_track']:
        print(f"      - {metric}")
    
    return results

if __name__ == "__main__":
    # Run demonstration
    demo_results = create_demo_analysis()
    
    # Save results
    with open('/workspace/limit_detection_framework_demo.json', 'w') as f:
        json.dump(demo_results, f, indent=2, default=str)
    
    print(f"\n💾 Demo results saved to: /workspace/limit_detection_framework_demo.json")
    
    print(f"\n" + "=" * 80)
    print("FRAMEWORK USAGE IN PRODUCTION")
    print("=" * 80)
    
    print("""
To use this framework with real Claude usage data:

1. INTEGRATE WITH CLAUDE_USAGE.PY:
   ```python
   from limit_detection_framework import LimitDetectionFramework
   
   # In ClaudeUsageAnalyzer class
   def check_limits(self):
       conversations = self.get_all_conversations()
       session_blocks = self.load_session_blocks(filter_recent=False)
       
       framework = LimitDetectionFramework(warning_threshold_percentage=0.9)
       results = framework.detect_limits_in_data(conversations, session_blocks)
       
       return results
   ```

2. ADD REAL-TIME MONITORING:
   ```python
   # In live monitoring
   def live_monitor_with_limits(self):
       while True:
           current_data = self.get_current_session_data()
           limit_check = framework.detect_limits_in_data([current_data])
           
           if limit_check['summary']['limit_hits_detected'] > 0:
               print("🚨 LIMIT HIT DETECTED!")
           elif limit_check['summary']['warnings_generated'] > 0:
               print("⚠️  APPROACHING LIMIT!")
   ```

3. ADD CLI COMMAND:
   ```bash
   claude-usage limits --check    # Check for limit patterns
   claude-usage limits --monitor  # Start real-time limit monitoring
   claude-usage limits --config   # Show monitoring configuration
   ```

4. IMPLEMENT ALERTS:
   - Email/Slack notifications for limit approaches
   - Dashboard warnings at 80%/90% thresholds  
   - Automatic conversation saving before limits
   - Model switching recommendations
   
5. INTEGRATION POINTS:
   - Session blocks: Look for limitReached, endReason fields
   - JSONL entries: Monitor token counts, conversation flow
   - Cost tracking: Unusual cost spikes indicate rapid consumption
   - Time patterns: Daily/hourly usage clustering analysis
""")