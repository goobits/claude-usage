#!/usr/bin/env python3
"""
Claude Usage Max - Fast Python implementation for Claude usage analysis across multiple VMs
"""
import os
import json
import glob
import argparse
import sys
import requests
from pathlib import Path
from datetime import datetime
from collections import defaultdict
from concurrent.futures import ProcessPoolExecutor, as_completed
import subprocess

class ClaudeUsageAnalyzer:
    def __init__(self, cost_mode='auto'):
        self.home_dir = Path.home()
        self.pricing_cache = None
        self.cost_mode = cost_mode  # 'auto', 'calculate', 'display'
        
    def get_pricing_data(self):
        """Fetch model pricing data from LiteLLM (cached)"""
        if self.pricing_cache is not None:
            return self.pricing_cache
            
        try:
            url = "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json"
            response = requests.get(url, timeout=10)
            response.raise_for_status()
            
            all_pricing = response.json()
            
            # Filter for Claude models only
            claude_pricing = {}
            for model_name, pricing in all_pricing.items():
                if model_name.startswith('claude-'):
                    claude_pricing[model_name] = {
                        'input_cost_per_token': pricing.get('input_cost_per_token'),
                        'output_cost_per_token': pricing.get('output_cost_per_token'),
                        'cache_creation_input_token_cost': pricing.get('cache_creation_input_token_cost'),
                        'cache_read_input_token_cost': pricing.get('cache_read_input_token_cost')
                    }
            
            self.pricing_cache = claude_pricing
            return claude_pricing
            
        except Exception as e:
            print(f"Warning: Could not fetch pricing data: {e}", file=sys.stderr)
            # Fallback pricing for common Claude models
            return {
                'claude-sonnet-4-20250514': {
                    'input_cost_per_token': 3e-06,  # $3 per 1M tokens
                    'output_cost_per_token': 1.5e-05,  # $15 per 1M tokens
                    'cache_creation_input_token_cost': None,
                    'cache_read_input_token_cost': None
                },
                'claude-opus-4-20250514': {
                    'input_cost_per_token': 1.5e-05,  # $15 per 1M tokens  
                    'output_cost_per_token': 7.5e-05,  # $75 per 1M tokens
                    'cache_creation_input_token_cost': None,
                    'cache_read_input_token_cost': None
                }
            }
    
    def calculate_cost_from_tokens(self, tokens, model_name):
        """Calculate cost based on token usage and model pricing"""
        pricing_data = self.get_pricing_data()
        
        if model_name not in pricing_data:
            return 0  # Unknown model
            
        pricing = pricing_data[model_name]
        cost = 0
        
        # Input tokens cost
        if pricing.get('input_cost_per_token') and tokens.get('input_tokens', 0):
            cost += tokens['input_tokens'] * pricing['input_cost_per_token']
            
        # Output tokens cost  
        if pricing.get('output_cost_per_token') and tokens.get('output_tokens', 0):
            cost += tokens['output_tokens'] * pricing['output_cost_per_token']
            
        # Cache creation tokens cost
        if (pricing.get('cache_creation_input_token_cost') and 
            tokens.get('cache_creation_input_tokens', 0)):
            cost += tokens['cache_creation_input_tokens'] * pricing['cache_creation_input_token_cost']
            
        # Cache read tokens cost
        if (pricing.get('cache_read_input_token_cost') and 
            tokens.get('cache_read_input_tokens', 0)):
            cost += tokens['cache_read_input_tokens'] * pricing['cache_read_input_token_cost']
            
        return cost
        
    def discover_claude_paths(self):
        """Fast discovery of Claude instances using os.listdir instead of glob"""
        paths = set()
        
        # Main Claude path
        main_path = self.home_dir / '.claude'
        if (main_path / 'projects').exists():
            paths.add(str(main_path))
        
        # VM paths - use os.listdir for speed
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
    
    
    def extract_session_info(self, session_dir_name):
        """Extract sessionId and projectPath like Node.js does"""
        session_id = session_dir_name
        
        if session_dir_name.startswith('-'):
            path_parts = session_dir_name[1:].split('-')  # Remove leading - and split
            project_name = path_parts[-1] if path_parts else 'unknown'  # Last part = project
        else:
            project_name = session_dir_name.split('/')[-1]
        
        return session_id, project_name
    
    def create_unique_hash(self, data):
        """Create unique hash for deduplication using messageId+requestId (same as Node.js)"""
        message_id = data.get('message', {}).get('id')
        request_id = data.get('requestId')
        
        if message_id is None or request_id is None:
            return None
            
        # Create hash using same logic as Node.js: messageId:requestId
        return f"{message_id}:{request_id}"
    
    def get_earliest_timestamp(self, jsonl_file):
        """Extract earliest timestamp from a JSONL file"""
        try:
            with open(jsonl_file, 'r') as f:
                earliest_date = None
                
                for line in f:
                    line = line.strip()
                    if not line:
                        continue
                        
                    try:
                        data = json.loads(line)
                        if 'timestamp' in data:
                            timestamp = data['timestamp']
                            # Handle both Z suffix and timezone info properly
                            if timestamp.endswith('Z'):
                                timestamp = timestamp[:-1] + '+00:00'
                            date = datetime.fromisoformat(timestamp)
                            # Convert to naive datetime for comparison
                            if date.tzinfo is not None:
                                date = date.replace(tzinfo=None)
                            if earliest_date is None or date < earliest_date:
                                earliest_date = date
                    except (json.JSONDecodeError, ValueError):
                        continue
                        
                return earliest_date
        except Exception:
            return None
    
    def sort_files_by_timestamp(self, jsonl_files):
        """Sort JSONL files by earliest timestamp (same as Node.js)"""
        files_with_timestamps = []
        
        for jsonl_file in jsonl_files:
            timestamp = self.get_earliest_timestamp(jsonl_file)
            files_with_timestamps.append((jsonl_file, timestamp))
        
        # Sort by timestamp (None values go to end)
        files_with_timestamps.sort(key=lambda x: x[1] if x[1] is not None else datetime.max)
        
        return [file for file, _ in files_with_timestamps]

    def calculate_entry_cost(self, data, current_usage, current_model):
        """Calculate cost for a single entry based on cost mode"""
        if self.cost_mode == 'display':
            # Always use costUSD, even if None
            return data.get('costUSD', 0) or 0
        elif self.cost_mode == 'calculate':
            # Always calculate from tokens
            if current_usage and current_model:
                return self.calculate_cost_from_tokens(current_usage, current_model)
            return 0
        else:  # 'auto' mode (default)
            # Prefer costUSD if available, otherwise calculate
            if 'costUSD' in data and data['costUSD'] is not None:
                return data['costUSD']
            elif current_usage and current_model:
                return self.calculate_cost_from_tokens(current_usage, current_model)
            return 0
    
    def sort_all_files_by_timestamp(self, file_tuples):
        """Sort all JSONL files by earliest timestamp globally"""
        files_with_timestamps = []
        
        for jsonl_file, session_dir in file_tuples:
            timestamp = self.get_earliest_timestamp(jsonl_file)
            files_with_timestamps.append((jsonl_file, session_dir, timestamp))
        
        # Sort by timestamp (None values go to end)
        files_with_timestamps.sort(key=lambda x: x[2] if x[2] is not None else datetime.max)
        
        return [(file, session_dir) for file, session_dir, _ in files_with_timestamps]
    
    def process_files_with_global_dedup(self, sorted_file_tuples, options):
        """Process all files with single global deduplication set (like Node.js)"""
        # Global deduplication set - exactly like Node.js
        global_processed_hashes = set()
        sessions_by_dir = defaultdict(lambda: {
            'total_cost': 0,
            'input_tokens': 0,
            'output_tokens': 0,
            'cache_creation_tokens': 0,
            'cache_read_tokens': 0,
            'last_activity': None,
            'models_used': set(),
            'session_dir': None
        })
        
        total_entries_processed = 0
        total_entries_skipped = 0
        
        # Process all files in chronological order with global deduplication
        for jsonl_file, session_dir in sorted_file_tuples:
            try:
                with open(jsonl_file, 'r') as f:
                    for line in f:
                        line = line.strip()
                        if not line:
                            continue
                        
                        try:
                            data = json.loads(line)
                            
                            # Check if this entry has usage data
                            if 'message' in data and 'usage' in data['message']:
                                total_entries_processed += 1
                                
                                # Create unique hash for deduplication (same as Node.js)
                                unique_hash = self.create_unique_hash(data)
                                
                                # Skip if we've already processed this messageId+requestId GLOBALLY
                                if unique_hash and unique_hash in global_processed_hashes:
                                    total_entries_skipped += 1
                                    continue  # Skip duplicate - already counted
                                
                                # Mark as processed GLOBALLY
                                if unique_hash:
                                    global_processed_hashes.add(unique_hash)
                                
                                # Extract usage data
                                usage = data['message']['usage']
                                current_input = usage.get('input_tokens', 0)
                                current_output = usage.get('output_tokens', 0)
                                current_cache_creation = usage.get('cache_creation_input_tokens', 0)
                                current_cache_read = usage.get('cache_read_input_tokens', 0)
                                
                                current_usage = {
                                    'input_tokens': current_input,
                                    'output_tokens': current_output,
                                    'cache_creation_input_tokens': current_cache_creation,
                                    'cache_read_input_tokens': current_cache_read
                                }
                                
                                # Extract model
                                current_model = None
                                if 'message' in data and 'model' in data['message']:
                                    current_model = data['message']['model']
                                
                                # Calculate cost based on mode
                                entry_cost = self.calculate_entry_cost(data, current_usage, current_model)
                                
                                # Extract timestamp for last activity (convert to local timezone like Node.js)
                                activity_date = None
                                if 'timestamp' in data:
                                    # Convert to local timezone like Node.js does
                                    timestamp = data['timestamp']
                                    if timestamp.endswith('Z'):
                                        timestamp = timestamp[:-1] + '+00:00'
                                    date_obj = datetime.fromisoformat(timestamp)
                                    # Convert to local timezone
                                    if date_obj.tzinfo is not None:
                                        local_date = date_obj.astimezone()
                                        activity_date = local_date.strftime('%Y-%m-%d')
                                    else:
                                        activity_date = date_obj.strftime('%Y-%m-%d')
                                
                                # Aggregate into session
                                session_data = sessions_by_dir[session_dir]
                                session_data['session_dir'] = session_dir
                                session_data['total_cost'] += entry_cost
                                session_data['input_tokens'] += current_input
                                session_data['output_tokens'] += current_output
                                session_data['cache_creation_tokens'] += current_cache_creation
                                session_data['cache_read_tokens'] += current_cache_read
                                
                                if current_model:
                                    session_data['models_used'].add(current_model)
                                
                                if activity_date and (session_data['last_activity'] is None or activity_date > session_data['last_activity']):
                                    session_data['last_activity'] = activity_date
                                    
                        except json.JSONDecodeError:
                            continue  # Skip invalid JSON lines
                            
            except Exception as e:
                if not options.get('json', False):
                    print(f"❌ Error processing {jsonl_file}: {e}")
                continue
        
        if not options.get('json', False):
            print(f"📊 Processed {total_entries_processed} entries, skipped {total_entries_skipped} duplicates")
        
        # Convert to final session format
        result = []
        for session_dir, session_data in sessions_by_dir.items():
            if session_data['total_cost'] == 0 and session_data['input_tokens'] == 0 and session_data['output_tokens'] == 0:
                continue  # Skip empty sessions
            
            # Extract session ID and project name from directory name
            session_dir_name = session_dir.name
            session_id, project_name = self.extract_session_info(session_dir_name)
            
            result.append({
                'sessionId': session_id,
                'projectPath': project_name,
                'inputTokens': session_data['input_tokens'],
                'outputTokens': session_data['output_tokens'],
                'cacheCreationTokens': session_data['cache_creation_tokens'],
                'cacheReadTokens': session_data['cache_read_tokens'],
                'totalCost': session_data['total_cost'],
                'lastActivity': session_data['last_activity'] or '1970-01-01',
                'modelsUsed': list(session_data['models_used'])
            })
        
        return result
    
    def aggregate_data_parallel(self, command, options=None):
        """Load and aggregate data from all Claude paths with global deduplication"""
        if options is None:
            options = {}
            
        paths = self.discover_claude_paths()
        
        if not options.get('json', False):
            print(f"🔍 Discovered {len(paths)} Claude instances")
        
        # Global deduplication - collect all files first, then process with single dedup set
        all_jsonl_files = []
        
        # Collect all JSONL files from all paths
        for claude_path in paths:
            try:
                projects_dir = Path(claude_path) / 'projects'
                if not projects_dir.exists():
                    continue
                    
                for session_dir in projects_dir.iterdir():
                    if not session_dir.is_dir():
                        continue
                    
                    jsonl_files = list(session_dir.glob('*.jsonl'))
                    for jsonl_file in jsonl_files:
                        all_jsonl_files.append((jsonl_file, session_dir))
                        
            except Exception as e:
                if not options.get('json', False):
                    print(f"❌ {claude_path}: {str(e)}")
        
        if not options.get('json', False):
            print(f"📁 Found {len(all_jsonl_files)} JSONL files across all instances")
        
        # Sort all files by timestamp globally (like Node.js does)
        sorted_files = self.sort_all_files_by_timestamp(all_jsonl_files)
        
        # Process with single global deduplication set
        return self.process_files_with_global_dedup(sorted_files, options)
    
    def process_daily_with_projects(self, session_data, limit=None):
        """Process session data into daily breakdown with project details"""
        date_groups = defaultdict(list)
        
        for session in session_data:
            # Convert lastActivity to local timezone like Node.js does
            last_activity = session.get('lastActivity', 'unknown')
            if last_activity and last_activity != 'unknown' and last_activity != '1970-01-01':
                try:
                    # lastActivity is already in YYYY-MM-DD format from session processing
                    date = last_activity
                except:
                    date = 'unknown'
            else:
                date = 'unknown'
            date_groups[date].append(session)
        
        result = []
        for date, sessions in date_groups.items():
            project_groups = defaultdict(lambda: {
                'project': '',
                'sessions': 0,
                'totalCost': 0,
                'totalTokens': 0
            })
            
            for session in sessions:
                # Extract project name from sessionId
                if session.get('sessionId') and session['sessionId'].startswith('-'):
                    path_parts = session['sessionId'][1:].split('-')
                    project_name = path_parts[-1] if path_parts else 'unknown'
                else:
                    project_name = session.get('projectPath', 'unknown').split('/')[-1]
                
                group = project_groups[project_name]
                group['project'] = project_name
                group['sessions'] += 1
                group['totalCost'] += session.get('totalCost', 0)
                group['totalTokens'] += (
                    session.get('inputTokens', 0) + 
                    session.get('outputTokens', 0) + 
                    session.get('cacheCreationTokens', 0) + 
                    session.get('cacheReadTokens', 0)
                )
            
            day_total = sum(s.get('totalCost', 0) for s in sessions)
            projects = sorted(project_groups.values(), key=lambda x: x['totalCost'], reverse=True)
            
            result.append({
                'date': date,
                'projects': projects,
                'totalCost': day_total,
                'totalSessions': len(sessions)
            })
        
        result.sort(key=lambda x: x['date'], reverse=True)
        
        # Apply limit
        display_limit = limit or 30
        return result[:display_limit]
    
    def process_monthly_data(self, daily_data, limit=None):
        """Aggregate daily data by month"""
        monthly_groups = defaultdict(lambda: {
            'month': '',
            'totalCost': 0,
            'totalSessions': 0
        })
        
        for day in daily_data:
            month = day['date'][:7]  # YYYY-MM
            group = monthly_groups[month]
            group['month'] = month
            group['totalCost'] += day['totalCost']
            group['totalSessions'] += day['totalSessions']
        
        result = sorted(monthly_groups.values(), key=lambda x: x['month'])
        
        # Apply limit
        display_limit = limit or 10
        return result[-display_limit:]  # Show most recent months
    
    def format_session_name(self, session):
        """Extract meaningful session name"""
        if session.get('sessionId') and session['sessionId'].startswith('-'):
            path_parts = session['sessionId'][1:].split('-')
            return path_parts[-1] if path_parts else 'unknown'
        elif session.get('projectPath') and session['projectPath'] != 'Unknown Project':
            return session['projectPath'].split('/')[-1]
        else:
            return session.get('sessionId') or session.get('projectPath') or 'Unknown Session'
    
    def display_daily(self, data, limit=None, json_output=False):
        """Display daily usage with project breakdown"""
        daily_data = self.process_daily_with_projects(data, limit)
        
        if json_output:
            print(json.dumps({"daily": daily_data}, indent=2))
            return
        
        print("\n" + "="*80)
        print("Claude Code Usage Report - Daily with Project Breakdown (All Instances)")
        print("="*80)
        
        total_cost = sum(day['totalCost'] for day in daily_data)
        total_sessions = sum(day['totalSessions'] for day in daily_data)
        
        print(f"\n📊 {len(daily_data)} days • {total_sessions} sessions • ${total_cost:.2f} total\n")
        
        for day in daily_data:
            print(f"📅 {day['date']} — ${day['totalCost']:.2f} ({day['totalSessions']} sessions)")
            
            for i, project in enumerate(day['projects'][:8]):  # Top 8 projects
                percentage = (project['totalCost'] / day['totalCost'] * 100) if day['totalCost'] > 0 else 0
                print(f"   {project['project']}: ${project['totalCost']:.2f} ({percentage:.0f}%, {project['sessions']} sessions)")
            
            if len(day['projects']) > 8:
                remaining_cost = sum(p['totalCost'] for p in day['projects'][8:])
                print(f"   ... {len(day['projects']) - 8} more: ${remaining_cost:.2f}")
            
            print()  # Empty line
    
    def display_monthly(self, data, limit=None, json_output=False):
        """Display monthly usage"""
        daily_data = self.process_daily_with_projects(data)
        monthly_data = self.process_monthly_data(daily_data, limit)
        
        if json_output:
            print(json.dumps({"monthly": monthly_data}, indent=2))
            return
        
        print("\n" + "="*80)
        print("Claude Code Usage Report - Monthly (All Instances)")
        print("="*80)
        
        total_cost = sum(month['totalCost'] for month in monthly_data)
        total_sessions = sum(month['totalSessions'] for month in monthly_data)
        
        print(f"\n📊 Total Usage Summary:")
        print(f"   Records: {len(monthly_data)}")
        print(f"   Total Cost: ${total_cost:.2f}")
        print(f"   Total Sessions: {total_sessions}")
        print()
        
        display_limit = limit or 10
        recent_data = monthly_data[-display_limit:]
        print(f"📅 Recent monthly usage (last {len(recent_data)}):")
        for month in recent_data:
            print(f"   {month['month']}: ${month['totalCost']:.2f} ({month['totalSessions']} sessions)")
    
    def display_session(self, data, limit=None, json_output=False):
        """Display session usage with timestamps"""
        # Sort by lastActivity (most recent first)
        sorted_data = sorted(data, key=lambda x: x.get('lastActivity', ''), reverse=True)
        
        if json_output:
            print(json.dumps({"session": sorted_data}, indent=2))
            return
        
        print("\n" + "="*80)
        print("Claude Code Usage Report - Session (All Instances)")
        print("="*80)
        
        total_cost = sum(s.get('totalCost', 0) for s in sorted_data)
        total_tokens = sum(
            s.get('inputTokens', 0) + s.get('outputTokens', 0) + 
            s.get('cacheCreationTokens', 0) + s.get('cacheReadTokens', 0) 
            for s in sorted_data
        )
        
        print(f"\n📊 Total Usage Summary:")
        print(f"   Records: {len(sorted_data)}")
        print(f"   Total Cost: ${total_cost:.2f}")
        print(f"   Total Tokens: {total_tokens:,}")
        print()
        
        display_limit = limit or 10
        recent_data = sorted_data[:display_limit]  # Take first N (most recent)
        print(f"📅 Recent session usage (last {len(recent_data)}):")
        for session in recent_data:
            session_name = self.format_session_name(session)
            timestamp = session.get('lastActivity', 'Unknown Date')
            tokens = (
                session.get('inputTokens', 0) + session.get('outputTokens', 0) + 
                session.get('cacheCreationTokens', 0) + session.get('cacheReadTokens', 0)
            )
            print(f"   {timestamp} | {session_name}: ${session.get('totalCost', 0):.2f} ({tokens:,} tokens)")
    
    def display_blocks(self, data, limit=None, json_output=False):
        """Display blocks usage"""
        if json_output:
            print(json.dumps({"blocks": data}, indent=2))
            return
        
        print("\n" + "="*80)
        print("Claude Code Usage Report - Blocks (All Instances)")
        print("="*80)
        
        total_cost = sum(block.get('costUSD', 0) for block in data)
        total_tokens = sum(
            (block.get('tokenCounts', {}).get('inputTokens', 0) + 
             block.get('tokenCounts', {}).get('outputTokens', 0) + 
             block.get('tokenCounts', {}).get('cacheCreationInputTokens', 0) + 
             block.get('tokenCounts', {}).get('cacheReadInputTokens', 0))
            for block in data
        )
        
        print(f"\n📊 Total Usage Summary:")
        print(f"   Records: {len(data)}")
        print(f"   Total Cost: ${total_cost:.2f}")
        print(f"   Total Tokens: {total_tokens:,}")
        print()
        
        display_limit = limit or 10
        recent_data = data[-display_limit:]  # Show most recent blocks
        print(f"📅 Recent blocks usage (last {len(recent_data)}):")
        for block in recent_data:
            start_time = datetime.fromisoformat(block['startTime'].replace('Z', '+00:00')).strftime('%m/%d/%Y, %I:%M:%S %p')
            tokens = (
                block.get('tokenCounts', {}).get('inputTokens', 0) + 
                block.get('tokenCounts', {}).get('outputTokens', 0) + 
                block.get('tokenCounts', {}).get('cacheCreationInputTokens', 0) + 
                block.get('tokenCounts', {}).get('cacheReadInputTokens', 0)
            )
            print(f"   {start_time}: ${block.get('costUSD', 0):.2f} ({tokens:,} tokens)")

def parse_args():
    """Parse command line arguments"""
    parser = argparse.ArgumentParser(description='Claude Usage Max - Fast Python implementation')
    
    # Commands
    parser.add_argument('command', nargs='?', default='daily',
                      choices=['daily', 'monthly', 'session', 'blocks'],
                      help='Command to run (default: daily)')
    
    # Options
    parser.add_argument('--json', action='store_true', help='Output in JSON format')
    parser.add_argument('--last', type=int, help='Show last N entries')
    parser.add_argument('--week', action='store_true', help='Show this week\'s data')
    parser.add_argument('--month', action='store_true', help='Show this month\'s data')
    parser.add_argument('--year', action='store_true', help='Show this year\'s data')
    parser.add_argument('--mode', choices=['auto', 'calculate', 'display'], 
                       default='auto', help='Cost calculation mode: auto (use costUSD if available, otherwise calculate), calculate (always calculate from tokens), display (always use costUSD)')
    
    # Month/year arguments
    months = ['january', 'february', 'march', 'april', 'may', 'june',
              'july', 'august', 'september', 'october', 'november', 'december']
    for month in months:
        parser.add_argument(f'--{month}', action='store_true', help=f'Show {month.title()} data')
    
    return parser.parse_args()

def main():
    args = parse_args()
    
    analyzer = ClaudeUsageAnalyzer(cost_mode=args.mode)
    
    # Build options based on arguments
    options = {}
    if args.json:
        options['json'] = True
    
    # Handle date filtering (simplified for now)
    if args.week:
        # Could implement week filtering here
        pass
    elif args.month:
        # Could implement month filtering here  
        pass
    elif args.year:
        # Could implement year filtering here
        pass
    
    try:
        # Load data
        data = analyzer.aggregate_data_parallel(args.command, options)
        
        if not data:
            if not args.json:
                print("No Claude usage data found across all instances.")
            else:
                print(json.dumps([]))
            return
        
        # Display results
        if args.command == 'daily':
            analyzer.display_daily(data, args.last, args.json)
        elif args.command == 'monthly':
            analyzer.display_monthly(data, args.last, args.json)
        elif args.command == 'session':
            analyzer.display_session(data, args.last, args.json)
        elif args.command == 'blocks':
            analyzer.display_blocks(data, args.last, args.json)
            
    except Exception as e:
        if args.json:
            print(json.dumps({"error": str(e)}))
        else:
            print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == '__main__':
    main()