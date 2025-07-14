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
    def __init__(self):
        self.home_dir = Path.home()
        self.pricing_cache = None
        
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
    
    def load_data_for_path(self, args):
        """Load data for a single Claude path - pure Python implementation"""
        claude_path, command, options = args
        
        try:
            projects_dir = Path(claude_path) / 'projects'
            if not projects_dir.exists():
                return {"path": claude_path, "error": "Projects directory not found", "count": 0}
            
            sessions = []
            
            # Each subdirectory is a project, each JSONL file in it is a session
            for project_dir in projects_dir.iterdir():
                if not project_dir.is_dir():
                    continue
                
                # Process all JSONL files in this project directory
                for jsonl_file in project_dir.glob('*.jsonl'):
                    session_data = self.parse_session_file(jsonl_file, project_dir.name)
                    if session_data:
                        sessions.append(session_data)
            
            return {"path": claude_path, "data": sessions, "count": len(sessions)}
            
        except Exception as e:
            return {"path": claude_path, "error": str(e), "count": 0}
    
    def parse_session_file(self, jsonl_file, project_dir_name):
        """Parse a single JSONL file representing one session"""
        try:
            total_cost = 0
            input_tokens = 0
            output_tokens = 0
            cache_creation_tokens = 0
            cache_read_tokens = 0
            last_activity = None
            models_used = set()
            
            with open(jsonl_file, 'r') as f:
                for line in f:
                    line = line.strip()
                    if not line:
                        continue
                    
                    try:
                        data = json.loads(line)
                        
                        # Extract usage data
                        current_usage = None
                        current_model = None
                        
                        if 'message' in data and 'usage' in data['message']:
                            usage = data['message']['usage']
                            current_input = usage.get('input_tokens', 0)
                            current_output = usage.get('output_tokens', 0)
                            current_cache_creation = usage.get('cache_creation_input_tokens', 0)
                            current_cache_read = usage.get('cache_read_input_tokens', 0)
                            
                            input_tokens += current_input
                            output_tokens += current_output
                            cache_creation_tokens += current_cache_creation
                            cache_read_tokens += current_cache_read
                            
                            current_usage = {
                                'input_tokens': current_input,
                                'output_tokens': current_output,
                                'cache_creation_input_tokens': current_cache_creation,
                                'cache_read_input_tokens': current_cache_read
                            }
                        
                        # Extract model
                        if 'message' in data and 'model' in data['message']:
                            current_model = data['message']['model']
                            models_used.add(current_model)
                        
                        # Calculate cost (prefer existing costUSD, otherwise calculate)
                        if 'costUSD' in data and data['costUSD'] is not None:
                            total_cost += data['costUSD']
                        elif current_usage and current_model:
                            calculated_cost = self.calculate_cost_from_tokens(current_usage, current_model)
                            total_cost += calculated_cost
                        
                        # Extract timestamp for last activity
                        if 'timestamp' in data:
                            activity_date = data['timestamp'][:10]  # YYYY-MM-DD
                            if last_activity is None or activity_date > last_activity:
                                last_activity = activity_date
                                
                    except json.JSONDecodeError:
                        continue  # Skip invalid JSON lines
            
            if input_tokens == 0 and output_tokens == 0 and total_cost == 0:
                return None  # Skip empty sessions
            
            # Extract project name from directory name (remove leading dash and get last part)
            if project_dir_name.startswith('-'):
                path_parts = project_dir_name[1:].split('-')
                project_name = path_parts[-1] if path_parts else 'unknown'
            else:
                project_name = project_dir_name.split('/')[-1]
            
            return {
                'sessionId': project_dir_name,
                'projectPath': project_name,
                'inputTokens': input_tokens,
                'outputTokens': output_tokens,
                'cacheCreationTokens': cache_creation_tokens,
                'cacheReadTokens': cache_read_tokens,
                'totalCost': total_cost,
                'lastActivity': last_activity or '1970-01-01',
                'modelsUsed': list(models_used)
            }
            
        except Exception as e:
            return None
    
    def aggregate_data_parallel(self, command, options=None):
        """Load and aggregate data from all Claude paths in parallel"""
        if options is None:
            options = {}
            
        paths = self.discover_claude_paths()
        
        if not options.get('json', False):
            print(f"🔍 Discovered {len(paths)} Claude instances")
        
        # Prepare arguments for parallel processing
        args_list = [(path, command, options) for path in paths]
        
        all_data = []
        
        # Use ProcessPoolExecutor for true parallelism
        with ProcessPoolExecutor(max_workers=min(len(paths), 8)) as executor:
            future_to_path = {executor.submit(self.load_data_for_path, args): args[0] 
                            for args in args_list}
            
            for future in as_completed(future_to_path):
                path = future_to_path[future]
                try:
                    result = future.result()
                    if 'data' in result and result['data']:
                        all_data.extend(result['data'])
                        if not options.get('json', False):
                            print(f"✓ {path}: {result['count']} records")
                    elif 'error' in result:
                        if not options.get('json', False):
                            print(f"❌ {path}: {result['error']}")
                except Exception as e:
                    if not options.get('json', False):
                        print(f"❌ {path}: {str(e)}")
        
        return all_data
    
    def process_daily_with_projects(self, session_data, limit=None):
        """Process session data into daily breakdown with project details"""
        date_groups = defaultdict(list)
        
        for session in session_data:
            date = session.get('lastActivity', 'unknown')[:10] if session.get('lastActivity') else 'unknown'
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
    
    # Month/year arguments
    months = ['january', 'february', 'march', 'april', 'may', 'june',
              'july', 'august', 'september', 'october', 'november', 'december']
    for month in months:
        parser.add_argument(f'--{month}', action='store_true', help=f'Show {month.title()} data')
    
    return parser.parse_args()

def main():
    args = parse_args()
    
    analyzer = ClaudeUsageAnalyzer()
    
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