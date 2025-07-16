#!/usr/bin/env python3
"""
Session Blocks Analysis - Analyze session block patterns for limit detection
"""
import os
import json
import glob
from pathlib import Path
from datetime import datetime, timedelta
from collections import defaultdict, Counter
import statistics

class SessionBlockAnalyzer:
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

    def find_session_block_files(self):
        """Find all session block files across all Claude paths"""
        paths = self.discover_claude_paths()
        block_files = []
        
        for claude_path in paths:
            try:
                # Check multiple possible locations for session blocks
                possible_dirs = [
                    Path(claude_path) / 'usage_tracking',
                    Path(claude_path),  # Sometimes stored in root
                    Path(claude_path) / 'sessions'  # Alternative location
                ]
                
                for usage_dir in possible_dirs:
                    if not usage_dir.exists():
                        continue
                    
                    # Find session blocks files
                    files = list(usage_dir.glob('session_blocks_*.json'))
                    files.extend(list(usage_dir.glob('*session_blocks*.json')))  # Alternative naming
                    
                    for f in files:
                        block_files.append({
                            'path': str(f),
                            'dir': str(usage_dir),
                            'size': f.stat().st_size,
                            'mtime': datetime.fromtimestamp(f.stat().st_mtime)
                        })
                        
            except Exception as e:
                print(f"Error processing {claude_path}: {e}")
                continue
        
        return block_files

    def load_all_session_blocks(self):
        """Load all session blocks and analyze their structure"""
        block_files = self.find_session_block_files()
        all_blocks = []
        file_analysis = []
        
        print(f"Found {len(block_files)} session block files:")
        for file_info in block_files:
            print(f"  {file_info['path']} ({file_info['size']} bytes, {file_info['mtime']})")
        
        for file_info in block_files:
            try:
                with open(file_info['path'], 'r') as f:
                    data = json.load(f)
                    
                    # Analyze file structure
                    analysis = {
                        'file': file_info['path'],
                        'file_size': file_info['size'],
                        'data_type': type(data).__name__,
                        'blocks_count': 0,
                        'sample_block': None,
                        'all_keys': set(),
                        'has_limit_fields': False,
                        'limit_indicators': []
                    }
                    
                    # Handle different data structures
                    file_blocks = []
                    if isinstance(data, list):
                        file_blocks = data
                        analysis['structure'] = 'array'
                    elif isinstance(data, dict):
                        analysis['structure'] = 'object'
                        analysis['top_level_keys'] = list(data.keys())
                        
                        if 'blocks' in data:
                            file_blocks = data['blocks']
                        elif 'sessions' in data:
                            file_blocks = data['sessions']
                        else:
                            # Treat the dict itself as a single block
                            file_blocks = [data]
                    
                    analysis['blocks_count'] = len(file_blocks)
                    
                    # Analyze individual blocks
                    for i, block in enumerate(file_blocks):
                        if isinstance(block, dict):
                            # Collect all keys
                            analysis['all_keys'].update(block.keys())
                            
                            # Check for limit-related fields
                            limit_fields = [
                                'limit', 'limits', 'maxTokens', 'tokenLimit', 'limitReached',
                                'limitHit', 'sessionEnded', 'endReason', 'terminationReason',
                                'hitLimit', 'maxReached', 'quotaExceeded', 'usage_limit'
                            ]
                            
                            for field in limit_fields:
                                if field in block:
                                    analysis['has_limit_fields'] = True
                                    analysis['limit_indicators'].append({
                                        'field': field,
                                        'value': block[field],
                                        'block_index': i
                                    })
                            
                            # Save first block as sample
                            if analysis['sample_block'] is None:
                                analysis['sample_block'] = block
                            
                            all_blocks.append(block)
                    
                    file_analysis.append(analysis)
                    
            except Exception as e:
                print(f"Error loading {file_info['path']}: {e}")
                continue
        
        return all_blocks, file_analysis

    def analyze_session_patterns(self, blocks):
        """Analyze session ending patterns and token distributions"""
        patterns = {
            'total_blocks': len(blocks),
            'token_distributions': {
                'input_tokens': [],
                'output_tokens': [],
                'total_tokens': []
            },
            'end_time_patterns': [],
            'session_durations': [],
            'potential_limit_thresholds': [],
            'abrupt_endings': [],
            'common_end_token_counts': Counter(),
            'session_end_analysis': []
        }
        
        for i, block in enumerate(blocks):
            if not isinstance(block, dict):
                continue
                
            analysis = {
                'block_index': i,
                'block_keys': list(block.keys()),
                'has_tokens': False,
                'has_timestamps': False,
                'potential_limit_hit': False,
                'end_characteristics': {}
            }
            
            # Extract token information
            token_fields = ['inputTokens', 'outputTokens', 'totalTokens', 'tokens', 'tokenCount']
            for field in token_fields:
                if field in block and isinstance(block[field], (int, float)):
                    patterns['token_distributions'][field.lower().replace('count', '_tokens')] = patterns['token_distributions'].get(field.lower().replace('count', '_tokens'), [])
                    patterns['token_distributions'][field.lower().replace('count', '_tokens')].append(block[field])
                    analysis['has_tokens'] = True
                    
                    # Check for common limit thresholds
                    token_value = block[field]
                    # Common Claude limits: 200k, 150k, 100k, 75k, 50k, 25k, 10k, 8k, 4k
                    common_limits = [200000, 150000, 100000, 75000, 50000, 25000, 10000, 8000, 4000]
                    for limit in common_limits:
                        if abs(token_value - limit) < (limit * 0.05):  # Within 5% of limit
                            analysis['potential_limit_hit'] = True
                            patterns['potential_limit_thresholds'].append({
                                'token_count': token_value,
                                'suspected_limit': limit,
                                'field': field,
                                'block_index': i
                            })
            
            # Extract timing information
            time_fields = ['startTime', 'endTime', 'timestamp', 'createdAt', 'lastActivity']
            for field in time_fields:
                if field in block:
                    analysis['has_timestamps'] = True
                    try:
                        timestamp = datetime.fromisoformat(str(block[field]).replace('Z', '+00:00'))
                        analysis['end_characteristics'][field] = timestamp
                    except:
                        pass
            
            # Calculate session duration if we have start and end times
            if 'startTime' in analysis['end_characteristics'] and 'endTime' in analysis['end_characteristics']:
                duration = (analysis['end_characteristics']['endTime'] - analysis['end_characteristics']['startTime']).total_seconds()
                patterns['session_durations'].append(duration)
                analysis['duration_seconds'] = duration
                
                # Very short sessions might indicate abrupt endings
                if duration < 300:  # Less than 5 minutes
                    patterns['abrupt_endings'].append(analysis)
            
            # Look for session count or active session indicators
            session_fields = ['sessionCount', 'activeSessionCount', 'sessions']
            for field in session_fields:
                if field in block:
                    analysis['end_characteristics'][field] = block[field]
            
            patterns['session_end_analysis'].append(analysis)
        
        return patterns

    def analyze_token_thresholds(self, patterns):
        """Analyze token count distributions to identify potential limit patterns"""
        threshold_analysis = {
            'suspicious_clustering': [],
            'potential_limits': [],
            'distribution_stats': {}
        }
        
        for token_type, values in patterns['token_distributions'].items():
            if not values:
                continue
                
            stats = {
                'count': len(values),
                'min': min(values),
                'max': max(values),
                'mean': statistics.mean(values),
                'median': statistics.median(values)
            }
            
            if len(values) > 1:
                stats['stdev'] = statistics.stdev(values)
            
            threshold_analysis['distribution_stats'][token_type] = stats
            
            # Look for clustering around potential limits
            value_counts = Counter(values)
            
            # Find values that appear frequently (potential limits)
            for value, count in value_counts.most_common(10):
                if count > 1:  # Appears more than once
                    # Check if it's near a common limit
                    common_limits = [200000, 150000, 100000, 75000, 50000, 25000, 10000, 8000, 4000]
                    for limit in common_limits:
                        if abs(value - limit) < (limit * 0.1):  # Within 10% of limit
                            threshold_analysis['potential_limits'].append({
                                'token_type': token_type,
                                'observed_value': value,
                                'frequency': count,
                                'suspected_limit': limit,
                                'percentage_of_limit': (value / limit) * 100
                            })
        
        return threshold_analysis

    def generate_report(self):
        """Generate comprehensive session blocks analysis report"""
        print("=" * 80)
        print("SESSION BLOCKS ANALYSIS REPORT")
        print("=" * 80)
        
        # Find session block files
        block_files = self.find_session_block_files()
        
        if not block_files:
            print("\n❌ NO SESSION BLOCK FILES FOUND")
            print("\nSearched locations:")
            paths = self.discover_claude_paths()
            for path in paths:
                print(f"  - {path}/usage_tracking/")
                print(f"  - {path}/")
                print(f"  - {path}/sessions/")
            
            print("\nNote: Session blocks may not exist if:")
            print("  1. Claude Code hasn't been used recently")
            print("  2. Session tracking is disabled")
            print("  3. Files are stored in a different location")
            print("  4. This is a development environment without real usage data")
            
            return {
                'status': 'no_files_found',
                'searched_paths': paths
            }
        
        # Load and analyze blocks
        print(f"\n📁 FOUND {len(block_files)} SESSION BLOCK FILES")
        all_blocks, file_analysis = self.load_all_session_blocks()
        
        if not all_blocks:
            print("\n❌ NO VALID SESSION BLOCKS LOADED")
            return {
                'status': 'no_blocks_loaded',
                'files_found': len(block_files)
            }
        
        print(f"\n📊 LOADED {len(all_blocks)} TOTAL SESSION BLOCKS")
        
        # Analyze file structures
        print("\n" + "=" * 40)
        print("FILE STRUCTURE ANALYSIS")
        print("=" * 40)
        
        for analysis in file_analysis:
            print(f"\n📄 {analysis['file']}")
            print(f"   Structure: {analysis['structure']}")
            print(f"   Blocks: {analysis['blocks_count']}")
            print(f"   All keys: {sorted(analysis['all_keys'])}")
            
            if analysis['has_limit_fields']:
                print(f"   🚨 LIMIT INDICATORS FOUND:")
                for indicator in analysis['limit_indicators']:
                    print(f"      - {indicator['field']}: {indicator['value']}")
            
            if analysis['sample_block']:
                print(f"   Sample block structure:")
                for key, value in analysis['sample_block'].items():
                    value_preview = str(value)[:50] + "..." if len(str(value)) > 50 else str(value)
                    print(f"      {key}: {value_preview}")
        
        # Analyze session patterns
        print("\n" + "=" * 40)
        print("SESSION PATTERN ANALYSIS")
        print("=" * 40)
        
        patterns = self.analyze_session_patterns(all_blocks)
        
        print(f"\n📈 TOKEN DISTRIBUTIONS:")
        for token_type, values in patterns['token_distributions'].items():
            if values:
                print(f"   {token_type.replace('_', ' ').title()}:")
                print(f"      Count: {len(values)}")
                print(f"      Range: {min(values)} - {max(values)}")
                print(f"      Average: {statistics.mean(values):.1f}")
                if len(values) > 1:
                    print(f"      Std Dev: {statistics.stdev(values):.1f}")
        
        print(f"\n⏱️  SESSION TIMING:")
        if patterns['session_durations']:
            durations = patterns['session_durations']
            print(f"   Sessions with timing: {len(durations)}")
            print(f"   Duration range: {min(durations):.1f}s - {max(durations):.1f}s")
            print(f"   Average duration: {statistics.mean(durations):.1f}s")
        
        if patterns['abrupt_endings']:
            print(f"   🚨 Abrupt endings (< 5 min): {len(patterns['abrupt_endings'])}")
        
        # Analyze potential limits
        print(f"\n🎯 LIMIT DETECTION ANALYSIS:")
        if patterns['potential_limit_thresholds']:
            print(f"   Potential limit hits found: {len(patterns['potential_limit_thresholds'])}")
            for threshold in patterns['potential_limit_thresholds']:
                print(f"      Token count: {threshold['token_count']} (suspected limit: {threshold['suspected_limit']})")
        else:
            print("   No obvious limit threshold patterns detected")
        
        # Token threshold analysis
        threshold_analysis = self.analyze_token_thresholds(patterns)
        
        if threshold_analysis['potential_limits']:
            print(f"\n🔍 CLUSTERING ANALYSIS:")
            for limit in threshold_analysis['potential_limits']:
                print(f"   {limit['token_type']}: {limit['observed_value']} tokens")
                print(f"      Frequency: {limit['frequency']} occurrences")
                print(f"      Suspected limit: {limit['suspected_limit']}")
                print(f"      Percentage of limit: {limit['percentage_of_limit']:.1f}%")
        
        # Recommendations
        print("\n" + "=" * 40)
        print("LIMIT DETECTION RECOMMENDATIONS")
        print("=" * 40)
        
        recommendations = []
        
        # Check if we found any limit indicators
        has_limit_fields = any(analysis['has_limit_fields'] for analysis in file_analysis)
        if has_limit_fields:
            recommendations.append("✅ Direct limit fields found in session blocks - use these for limit detection")
        else:
            recommendations.append("❌ No direct limit fields found in session blocks")
        
        # Check clustering patterns
        if threshold_analysis['potential_limits']:
            recommendations.append("✅ Token clustering patterns suggest potential limits - analyze high-frequency token counts")
        
        # Check for abrupt endings
        if patterns['abrupt_endings']:
            recommendations.append("⚠️  Abrupt session endings detected - may indicate limit hits")
        
        # General recommendations
        recommendations.extend([
            "🔍 Monitor sessions ending near common Claude limits (200k, 100k, 75k, 50k tokens)",
            "📊 Track token count distributions over time to identify usage patterns",
            "⏱️  Analyze session duration vs token count relationships",
            "🚨 Implement alerts for sessions approaching known token thresholds"
        ])
        
        for i, rec in enumerate(recommendations, 1):
            print(f"{i}. {rec}")
        
        return {
            'status': 'success',
            'files_analyzed': len(file_analysis),
            'blocks_loaded': len(all_blocks),
            'has_limit_fields': has_limit_fields,
            'potential_limits': threshold_analysis['potential_limits'],
            'patterns': patterns,
            'file_analysis': file_analysis,
            'recommendations': recommendations
        }

if __name__ == "__main__":
    analyzer = SessionBlockAnalyzer()
    result = analyzer.generate_report()
    
    # Save detailed results
    with open('/workspace/session_blocks_analysis_results.json', 'w') as f:
        # Convert datetime objects to strings for JSON serialization
        def json_serializer(obj):
            if isinstance(obj, datetime):
                return obj.isoformat()
            return str(obj)
        
        json.dump(result, f, indent=2, default=json_serializer)
    
    print(f"\n💾 Detailed results saved to: /workspace/session_blocks_analysis_results.json")