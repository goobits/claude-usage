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
import time
import signal
from pathlib import Path
from datetime import datetime, timedelta
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
    
    def get_file_date_range(self, jsonl_file):
        """Get earliest and latest timestamps from a JSONL file for date filtering"""
        try:
            with open(jsonl_file, 'r') as f:
                earliest_date = None
                latest_date = None
                
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
                            if latest_date is None or date > latest_date:
                                latest_date = date
                    except (json.JSONDecodeError, ValueError):
                        continue
                        
                return earliest_date, latest_date
        except Exception:
            return None, None
    
    def should_include_file(self, jsonl_file, since_date=None, until_date=None):
        """Check if file should be included based on date range"""
        if since_date is None and until_date is None:
            return True
            
        # Use file modification time as fast pre-filter
        try:
            file_mtime = datetime.fromtimestamp(jsonl_file.stat().st_mtime)
            
            # Quick exclusion: if file is older than since_date, skip parsing
            if since_date and file_mtime < since_date:
                return False
                
            # Quick exclusion: if file is newer than until_date + 1 day, skip parsing  
            if until_date:
                until_plus_day = until_date.replace(hour=23, minute=59, second=59)
                if file_mtime > until_plus_day:
                    return False
        except Exception:
            pass  # Fall back to content parsing if file time check fails
        
        # If file time suggests it might contain relevant data, parse first/last lines
        earliest, latest = self.get_file_date_range(jsonl_file)
        
        if earliest is None and latest is None:
            return False  # No timestamps found
            
        # Check if file date range overlaps with requested range
        if since_date and latest and latest.date() < since_date.date():
            return False
        if until_date and earliest and earliest.date() > until_date.date():
            return False
            
        return True
    
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
        """Sort all JSONL files by file modification time, then earliest timestamp"""
        files_with_timestamps = []
        
        for jsonl_file, session_dir in file_tuples:
            # Use file modification time as primary sort (much faster)
            try:
                file_mtime = datetime.fromtimestamp(jsonl_file.stat().st_mtime)
            except Exception:
                file_mtime = datetime.max  # Put files we can't read at the end
            
            # Only parse content timestamp if file times are very close (within 1 hour)
            # This is much faster than parsing every file
            content_timestamp = None
            try:
                # For most cases, file mtime is sufficient for sorting
                # Only parse content if we really need fine-grained ordering
                if len(file_tuples) < 100:  # Only for smaller datasets
                    content_timestamp = self.get_earliest_timestamp(jsonl_file)
            except Exception:
                pass
            
            # Use file mtime as primary sort key, content timestamp as secondary
            primary_sort = file_mtime
            secondary_sort = content_timestamp if content_timestamp else file_mtime
            
            files_with_timestamps.append((jsonl_file, session_dir, primary_sort, secondary_sort))
        
        # Sort by file modification time primarily, content timestamp secondarily
        files_with_timestamps.sort(key=lambda x: (x[2], x[3]))
        
        return [(file, session_dir) for file, session_dir, _, _ in files_with_timestamps]
    
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
        
        # Lazy processing: determine what data we need to track
        command = options.get('command', 'daily')
        need_timestamps = command in ['daily', 'session', 'monthly']  # Monthly needs timestamps for month grouping
        
        # Early exit optimization for --last N queries
        last_limit = options.get('last')
        session_count = 0
        should_stop_early = False
        
        # Optimized deduplication: only track recent hashes for time-window overlap
        # Claude Code only creates duplicates within short time windows (conversation branches)
        dedup_window_hours = 24  # Only deduplicate within 24-hour windows
        dedup_cleanup_threshold = 10000  # Clean up old hashes periodically
        hash_timestamps = {}  # Track when each hash was first seen
        
        total_entries_processed = 0
        total_entries_skipped = 0
        
        # Process all files in chronological order with global deduplication
        for jsonl_file, session_dir in sorted_file_tuples:
            # Early exit optimization: if we have enough sessions for --last N, stop
            if last_limit and command == 'session' and session_count >= last_limit:
                should_stop_early = True
                break
                
            try:
                # Stream processing: read line by line instead of loading entire file
                with open(jsonl_file, 'r') as f:
                    has_session_data = False
                    
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
                                
                                # Get current entry timestamp for dedup window optimization
                                current_timestamp = None
                                if 'timestamp' in data:
                                    try:
                                        timestamp = data['timestamp']
                                        if timestamp.endswith('Z'):
                                            timestamp = timestamp[:-1] + '+00:00'
                                        current_timestamp = datetime.fromisoformat(timestamp)
                                        if current_timestamp.tzinfo is not None:
                                            current_timestamp = current_timestamp.replace(tzinfo=None)
                                    except Exception:
                                        pass
                                
                                # Optimized deduplication: only check within time window
                                skip_duplicate = False
                                if unique_hash and unique_hash in global_processed_hashes:
                                    # Check if the hash was seen within the deduplication window
                                    if unique_hash in hash_timestamps and current_timestamp:
                                        time_diff = abs((current_timestamp - hash_timestamps[unique_hash]).total_seconds() / 3600)
                                        if time_diff <= dedup_window_hours:
                                            skip_duplicate = True
                                    else:
                                        skip_duplicate = True  # Always skip if no timestamp available
                                
                                if skip_duplicate:
                                    total_entries_skipped += 1
                                    continue  # Skip duplicate - already counted
                                
                                # Mark as processed GLOBALLY with timestamp
                                if unique_hash:
                                    global_processed_hashes.add(unique_hash)
                                    if current_timestamp:
                                        hash_timestamps[unique_hash] = current_timestamp
                                
                                # Periodic cleanup of old dedup hashes to save memory
                                if len(hash_timestamps) > dedup_cleanup_threshold and current_timestamp:
                                    cutoff_time = current_timestamp - timedelta(hours=dedup_window_hours * 2)
                                    old_hashes = [h for h, t in hash_timestamps.items() if t < cutoff_time]
                                    for old_hash in old_hashes:
                                        hash_timestamps.pop(old_hash, None)
                                        global_processed_hashes.discard(old_hash)
                                
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
                                
                                # Extract timestamp for last activity (only if needed)
                                activity_date = None
                                if need_timestamps and 'timestamp' in data:
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
                                
                                # Only update timestamps if needed for this command
                                if need_timestamps and activity_date and (session_data['last_activity'] is None or activity_date > session_data['last_activity']):
                                    session_data['last_activity'] = activity_date
                                
                                has_session_data = True
                                    
                        except json.JSONDecodeError:
                            continue  # Skip invalid JSON lines
                    
                    # Count sessions for early exit optimization
                    if has_session_data and session_dir not in [s['session_dir'] for s in sessions_by_dir.values()]:
                        session_count += 1
                            
            except Exception as e:
                if not options.get('json', False):
                    print(f"❌ Error processing {jsonl_file}: {e}")
                continue
        
        if not options.get('json', False):
            status_msg = f"📊 Processed {total_entries_processed} entries, skipped {total_entries_skipped} duplicates"
            if should_stop_early:
                status_msg += f" (early exit after {session_count} sessions)"
            print(status_msg)
        
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
                'modelsUsed': sorted(list(session_data['models_used']))
            })
        
        return result
    
    def aggregate_data_parallel(self, command, options=None):
        """Load and aggregate data from all Claude paths with global deduplication"""
        if options is None:
            options = {}
            
        # Parse date filters
        since_date = None
        until_date = None
        if options.get('since'):
            try:
                since_date = datetime.strptime(options['since'], '%Y-%m-%d')
            except ValueError:
                if not options.get('json', False):
                    print(f"❌ Invalid since date format: {options['since']}. Use YYYY-MM-DD")
                return []
        
        if options.get('until'):
            try:
                until_date = datetime.strptime(options['until'], '%Y-%m-%d')
            except ValueError:
                if not options.get('json', False):
                    print(f"❌ Invalid until date format: {options['until']}. Use YYYY-MM-DD")
                return []
            
        paths = self.discover_claude_paths()
        
        if not options.get('json', False):
            print(f"🔍 Discovered {len(paths)} Claude instances")
        
        # Global deduplication - collect all files first, then filter by date
        all_jsonl_files = []
        files_filtered = 0
        
        # Collect all JSONL files from all paths with date filtering
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
                        # Pre-filter files by date range for massive speedup
                        if self.should_include_file(jsonl_file, since_date, until_date):
                            all_jsonl_files.append((jsonl_file, session_dir))
                        else:
                            files_filtered += 1
                        
            except Exception as e:
                if not options.get('json', False):
                    print(f"❌ {claude_path}: {str(e)}")
        
        if not options.get('json', False):
            if files_filtered > 0:
                print(f"📁 Found {len(all_jsonl_files)} JSONL files (filtered out {files_filtered} by date)")
            else:
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

    # Live monitoring functionality
    def create_progress_bar(self, percentage, width=20, status_color='🟢'):
        """Create progress bar with cursor and color-coded status"""
        pct = max(0, min(100, percentage))
        
        if pct >= 100:
            filled = width
            return f"{status_color} {'█' * filled}"
        else:
            filled = int(width * pct / 100)
            cursor = 1 if pct < 100 else 0
            empty = max(0, width - filled - cursor)
            
            filled_bar = '█' * filled
            cursor_char = '▓' if cursor else ''
            empty_bar = '░' * empty
            
            return f"{status_color} {filled_bar}{cursor_char}{empty_bar}"
    
    def format_time(self, minutes):
        """Format time duration for display"""
        if minutes < 60:
            return f"{int(minutes)}m"
        hours = int(minutes // 60)
        mins = int(minutes % 60)
        if mins == 0:
            return f"{hours}h"
        return f"{hours}h {mins}m"
    
    def clear_screen(self):
        """Clear terminal screen"""
        if sys.stdout.isatty():
            os.system('clear' if os.name == 'posix' else 'cls')
    
    def hide_cursor(self):
        """Hide terminal cursor"""
        if sys.stdout.isatty():
            sys.stdout.write('\033[?25l')
            sys.stdout.flush()
    
    def show_cursor(self):
        """Show terminal cursor"""
        if sys.stdout.isatty():
            sys.stdout.write('\033[?25h')
            sys.stdout.flush()
    
    def load_session_blocks(self, filter_recent=True):
        """Load session blocks (optionally filter to last 24 hours for live monitoring)"""
        paths = self.discover_claude_paths()
        blocks = []
        cutoff_time = datetime.now() - timedelta(hours=24) if filter_recent else None
        
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
                    
                    # Find session blocks files (only recent ones)
                    block_files = list(usage_dir.glob('session_blocks_*.json'))
                    block_files.extend(list(usage_dir.glob('*session_blocks*.json')))  # Alternative naming
                    
                    # Filter by file modification time if requested
                    if filter_recent and cutoff_time:
                        files_to_process = [f for f in block_files 
                                          if datetime.fromtimestamp(f.stat().st_mtime) > cutoff_time]
                    else:
                        files_to_process = block_files
                    
                    for block_file in files_to_process:
                        try:
                            with open(block_file, 'r') as f:
                                data = json.load(f)
                                
                                # Handle different data structures
                                file_blocks = []
                                if isinstance(data, list):
                                    file_blocks = data
                                elif isinstance(data, dict):
                                    if 'blocks' in data:
                                        file_blocks = data['blocks']
                                    elif 'sessions' in data:
                                        file_blocks = data['sessions']
                                
                                # Filter blocks to only recent ones if requested
                                for block in file_blocks:
                                    try:
                                        if filter_recent and cutoff_time and 'startTime' in block:
                                            start_time = datetime.fromisoformat(block['startTime'].replace('Z', '+00:00'))
                                            if start_time.replace(tzinfo=None) > cutoff_time:
                                                blocks.append(block)
                                        else:
                                            blocks.append(block)
                                    except Exception:
                                        continue
                                        
                        except Exception:
                            continue
                        
            except Exception:
                continue  # Skip failed paths silently
        
        return blocks
    
    def get_current_session_data(self):
        """Get current session data from ALL recently active sessions across all VMs"""
        paths = self.discover_claude_paths()
        cutoff_time = datetime.now() - timedelta(minutes=10)  # Last 10 minutes for "current"
        current_sessions = {}
        
        # Find ALL recently modified JSONL files across all paths
        recent_files = []
        
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
                        try:
                            file_mtime = datetime.fromtimestamp(jsonl_file.stat().st_mtime)
                            if file_mtime > cutoff_time:
                                recent_files.append((jsonl_file, session_dir, file_mtime))
                        except Exception:
                            continue
            except Exception:
                continue
        
        # Process ALL recently active sessions
        for jsonl_file, session_dir, file_mtime in recent_files:
            session_data = {
                'sessionId': session_dir.name,
                'total_cost': 0,
                'total_tokens': 0,
                'start_time': None,
                'last_activity': None,
                'file_modified': file_mtime
            }
            
            # For live monitoring, only show very recent activity (last 2 minutes)
            # But track actual timing per entry for accurate burn rate calculation
            now_utc = datetime.utcnow()
            utc_cutoff = now_utc - timedelta(minutes=2)
            
            # Track tokens by time for accurate burn rate
            tokens_by_minute = defaultdict(int)
            cost_by_minute = defaultdict(float)
            models_seen = set()  # Track actual models being used
            token_cost_by_model = defaultdict(lambda: {'tokens': 0, 'cost': 0})  # Track per-model usage
            
            try:
                with open(jsonl_file, 'r') as f:
                    for line in f:
                        line = line.strip()
                        if not line:
                            continue
                            
                        try:
                            data = json.loads(line)
                            
                            # Check timestamp and extract entry time
                            entry_time = None
                            if 'timestamp' in data:
                                timestamp = data['timestamp']
                                if timestamp.endswith('Z'):
                                    # UTC timestamp
                                    entry_time = datetime.fromisoformat(timestamp[:-1])
                                else:
                                    # Assume UTC if no timezone
                                    entry_time = datetime.fromisoformat(timestamp.replace('+00:00', ''))
                                
                                if entry_time < utc_cutoff:
                                    continue  # Skip older entries
                                
                                # Track timing (keep in UTC for consistency)
                                if session_data['start_time'] is None or entry_time < session_data['start_time']:
                                    session_data['start_time'] = entry_time
                                if session_data['last_activity'] is None or entry_time > session_data['last_activity']:
                                    session_data['last_activity'] = entry_time
                            
                            # Extract usage data
                            if 'message' in data and 'usage' in data['message'] and entry_time:
                                usage = data['message']['usage']
                                
                                tokens = (usage.get('input_tokens', 0) + 
                                        usage.get('output_tokens', 0) + 
                                        usage.get('cache_creation_input_tokens', 0) + 
                                        usage.get('cache_read_input_tokens', 0))
                                
                                session_data['total_tokens'] += tokens
                                
                                # Track tokens by minute for accurate burn rate calculation
                                # Use actual conversation time, not file processing time
                                minute_key = entry_time.strftime('%Y-%m-%d %H:%M')
                                tokens_by_minute[minute_key] += tokens
                                
                                # Track actual model being used and calculate cost
                                model_name = None
                                if 'model' in data['message']:
                                    model_name = data['message']['model']
                                    models_seen.add(model_name)
                                
                                # Calculate cost
                                entry_cost = 0
                                if 'costUSD' in data and data['costUSD'] is not None:
                                    entry_cost = data['costUSD']
                                elif model_name:
                                    entry_cost = self.calculate_cost_from_tokens({
                                        'input_tokens': usage.get('input_tokens', 0),
                                        'output_tokens': usage.get('output_tokens', 0),
                                        'cache_creation_input_tokens': usage.get('cache_creation_input_tokens', 0),
                                        'cache_read_input_tokens': usage.get('cache_read_input_tokens', 0)
                                    }, model_name)
                                
                                # Track per-model usage for cost calculation accuracy
                                if model_name:
                                    token_cost_by_model[model_name]['tokens'] += tokens
                                    token_cost_by_model[model_name]['cost'] += entry_cost
                                
                                session_data['total_cost'] += entry_cost
                                cost_by_minute[minute_key] += entry_cost
                                    
                        except json.JSONDecodeError:
                            continue
                
                # Only include if it has recent data
                if (session_data['total_tokens'] > 0 or session_data['total_cost'] > 0):
                    # Add burn rate calculation data
                    session_data['tokens_by_minute'] = dict(tokens_by_minute)
                    session_data['cost_by_minute'] = dict(cost_by_minute)
                    session_data['models_used'] = list(models_seen)
                    session_data['usage_by_model'] = dict(token_cost_by_model)
                    
                    # Calculate burn rates from actual conversation timeline
                    if len(tokens_by_minute) >= 1:
                        recent_minutes = sorted(tokens_by_minute.keys())
                        recent_tokens = sum(tokens_by_minute[m] for m in recent_minutes)
                        recent_cost = sum(cost_by_minute.get(m, 0) for m in recent_minutes)
                        
                        # Calculate based on actual conversation timespan, not just number of minutes with data
                        if len(recent_minutes) >= 2:
                            # Calculate actual time span of conversation
                            start_minute = datetime.strptime(recent_minutes[0], '%Y-%m-%d %H:%M')
                            end_minute = datetime.strptime(recent_minutes[-1], '%Y-%m-%d %H:%M')
                            actual_duration_minutes = (end_minute - start_minute).total_seconds() / 60 + 1  # +1 for inclusive
                            
                            # Use actual conversation duration for burn rate
                            burn_rate_tokens = recent_tokens / actual_duration_minutes
                            burn_rate_cost = (recent_cost / actual_duration_minutes) * 60  # per hour
                        else:
                            # Single minute of data - use that minute's rate
                            burn_rate_tokens = recent_tokens
                            burn_rate_cost = recent_cost * 60  # per hour
                        
                        # Use actual calculated burn rates - no artificial caps
                        session_data['realBurnRateTokens'] = burn_rate_tokens
                        session_data['realBurnRateCost'] = burn_rate_cost
                    else:
                        # No data available
                        session_data['realBurnRateTokens'] = 0
                        session_data['realBurnRateCost'] = 0
                    
                    current_sessions[session_dir.name] = session_data
                    
            except Exception:
                pass
                
        return current_sessions
    
    def find_active_session_block(self, cached_blocks=None, cache_time=None):
        """Find currently active session block with 30s caching, fallback to current session data"""
        current_time = time.time()
        
        # Use cache if available and recent (30 seconds)
        if cached_blocks is not None and cache_time is not None:
            if current_time - cache_time < 30:
                # Find active block from cache
                now = datetime.now()
                for block in cached_blocks:
                    try:
                        end_time = datetime.fromisoformat(block['endTime'].replace('Z', '+00:00'))
                        if end_time.replace(tzinfo=None) > now:
                            return block, cached_blocks, cache_time
                    except Exception:
                        continue
                return None, cached_blocks, cache_time
        
        # Try to load session blocks first
        blocks = self.load_session_blocks()
        now = datetime.now()
        
        # Find active block from session blocks
        for block in blocks:
            try:
                end_time = datetime.fromisoformat(block['endTime'].replace('Z', '+00:00'))
                if end_time.replace(tzinfo=None) > now:
                    return block, blocks, current_time
            except Exception:
                continue
        
        # Fallback: if no session blocks found, create aggregated synthetic block from ALL current sessions
        current_sessions = self.get_current_session_data()
        if current_sessions:
            # Aggregate data from ALL active sessions
            total_tokens = 0
            total_cost = 0
            earliest_start = None
            latest_activity = None
            active_session_count = len(current_sessions)
            
            # Aggregate per-minute data for accurate burn rate calculation
            all_tokens_by_minute = defaultdict(int)
            all_cost_by_minute = defaultdict(float)
            total_burn_rate_tokens = 0
            total_burn_rate_cost = 0
            sessions_with_valid_burn_rates = 0
            
            for session_id, session_data in current_sessions.items():
                total_tokens += session_data['total_tokens']
                total_cost += session_data['total_cost']
                
                # Use the real burn rates calculated from actual usage
                session_burn_tokens = session_data.get('realBurnRateTokens', 0)
                session_burn_cost = session_data.get('realBurnRateCost', 0)
                total_burn_rate_tokens += session_burn_tokens
                total_burn_rate_cost += session_burn_cost
                if session_burn_tokens > 0:
                    sessions_with_valid_burn_rates += 1
                
                if session_data['start_time']:
                    if earliest_start is None or session_data['start_time'] < earliest_start:
                        earliest_start = session_data['start_time']
                
                if session_data['last_activity']:
                    if latest_activity is None or session_data['last_activity'] > latest_activity:
                        latest_activity = session_data['last_activity']
                
                # Aggregate per-minute burn rate data (for backup calculation)
                for minute, tokens in session_data.get('tokens_by_minute', {}).items():
                    all_tokens_by_minute[minute] += tokens
                for minute, cost in session_data.get('cost_by_minute', {}).items():
                    all_cost_by_minute[minute] += cost
            
            if latest_activity and total_tokens > 0:
                # Use the aggregated burn rates from actual conversation timelines
                burn_rate_tokens = total_burn_rate_tokens
                burn_rate_cost = total_burn_rate_cost
                
                # No need for artificial adjustments - we're using actual conversation timelines
                
                # Create aggregated synthetic session block representing all active sessions
                start_time = earliest_start or latest_activity
                # Set end time to be 10 minutes after latest activity
                end_time = latest_activity + timedelta(minutes=10)
                
                synthetic_block = {
                    'startTime': start_time.isoformat() + 'Z',
                    'endTime': end_time.isoformat() + 'Z',
                    'tokenCounts': {
                        'inputTokens': total_tokens // 2,  # Rough split
                        'outputTokens': total_tokens // 2,
                        'cacheCreationInputTokens': 0,
                        'cacheReadInputTokens': 0
                    },
                    'costUSD': total_cost,
                    'activeSessionCount': active_session_count,
                    # Add real burn rate data
                    'realBurnRateTokens': burn_rate_tokens,
                    'realBurnRateCost': burn_rate_cost,
                    'tokensPerMinute': dict(all_tokens_by_minute)  # For debugging
                }
                return synthetic_block, [synthetic_block], current_time
        
        return None, blocks, current_time
    
    def run_live_monitor(self, snapshot=False, json_output=False):
        """Run live monitoring with performance optimizations"""
        TOKEN_LIMIT = 880000  # Max20 limit
        BUDGET_LIMIT = TOKEN_LIMIT * 0.0015  # ~$1.50 per 1000 tokens
        
        # Performance cache
        cached_blocks = None
        cache_time = None
        
        def display_live_data():
            """Display live monitoring data once"""
            # Find active session with caching
            nonlocal cached_blocks, cache_time
            active_block, cached_blocks, cache_time = self.find_active_session_block(cached_blocks, cache_time)
            
            now = datetime.now()
            current_time = now.strftime('%H:%M')
            
            if json_output:
                # JSON output for snapshot mode
                if active_block:
                    token_counts = active_block.get('tokenCounts', {})
                    total_tokens = sum(token_counts.values())
                    cost_used = active_block.get('costUSD', 0)
                    
                    try:
                        start_time = datetime.fromisoformat(active_block['startTime'].replace('Z', ''))
                        end_time = datetime.fromisoformat(active_block['endTime'].replace('Z', ''))
                        now_utc = datetime.utcnow()
                        
                        elapsed_minutes = (now_utc - start_time).total_seconds() / 60
                        remaining_minutes = max(0, (end_time - now_utc).total_seconds() / 60)
                        
                        # Use real burn rates if available
                        if 'realBurnRateTokens' in active_block and 'realBurnRateCost' in active_block:
                            burn_rate = active_block['realBurnRateTokens']
                            cost_burn_rate = active_block['realBurnRateCost']
                        else:
                            burn_rate = total_tokens / elapsed_minutes if elapsed_minutes > 0 else 0
                            cost_burn_rate = (cost_used / elapsed_minutes) * 60 if elapsed_minutes > 0 else 0
                        
                        snapshot_data = {
                            'status': 'active',
                            'tokens': {
                                'current': total_tokens,
                                'limit': TOKEN_LIMIT,
                                'percentage': (total_tokens / TOKEN_LIMIT) * 100
                            },
                            'cost': {
                                'current': cost_used,
                                'limit': BUDGET_LIMIT
                            },
                            'timing': {
                                'elapsed_minutes': elapsed_minutes,
                                'remaining_minutes': remaining_minutes,
                                'current_time': current_time
                            },
                            'burn_rates': {
                                'tokens_per_minute': burn_rate,
                                'cost_per_hour': cost_burn_rate
                            },
                            'session_count': active_block.get('activeSessionCount', 1)
                        }
                    except Exception:
                        snapshot_data = {'status': 'error', 'message': 'Could not parse session data'}
                else:
                    snapshot_data = {'status': 'inactive', 'message': 'No active session'}
                
                print(json.dumps(snapshot_data, indent=2))
                return
            
            # Terminal output
            if not snapshot:
                self.clear_screen()
            
            # Print header
            print("\033[1m[ CLAUDE USAGE MONITOR ]\033[0m")
            print()
            
            if not active_block:
                # No active session - show waiting state
                print(f"⚡ Tokens:  {self.create_progress_bar(0, 20, '🟢')} 0 / {TOKEN_LIMIT:,}")
                print(f"💲 Budget:  {self.create_progress_bar(0, 20, '🟢')} $0.00 / ${BUDGET_LIMIT:.2f}")
                print(f"♻️  Reset:   {self.create_progress_bar(0, 20, '🕐')} 0m")
                print()
                print("🔥 0.0 tok/min | 💰 $0.00/hour")
                print()
                print(f"🕐 {current_time} | 🏁 No session | ♻️  Next reset")
                print()
                print("📝 No active session")
                if snapshot:
                    print(f"\n[Snapshot mode - scanned {len(self.discover_claude_paths())} Claude instances]")
            else:
                # Active session - calculate metrics
                try:
                    start_time = datetime.fromisoformat(active_block['startTime'].replace('Z', ''))
                    end_time = datetime.fromisoformat(active_block['endTime'].replace('Z', ''))
                    
                    # Get token counts
                    token_counts = active_block.get('tokenCounts', {})
                    total_tokens = (
                        token_counts.get('inputTokens', 0) +
                        token_counts.get('outputTokens', 0) +
                        token_counts.get('cacheCreationInputTokens', 0) +
                        token_counts.get('cacheReadInputTokens', 0)
                    )
                    
                    cost_used = active_block.get('costUSD', 0)
                    
                    # Calculate session progress
                    now_utc = datetime.utcnow()
                    total_session_minutes = (end_time - start_time).total_seconds() / 60
                    elapsed_minutes = max(0, (now_utc - start_time).total_seconds() / 60)
                    remaining_minutes = max(0, (end_time - now_utc).total_seconds() / 60)
                    
                    # Progress percentages
                    token_percentage = (total_tokens / TOKEN_LIMIT) * 100
                    token_status = '🟢' if token_percentage < 70 else '🟡' if token_percentage < 90 else '🔴'
                    
                    budget_percentage = (cost_used / BUDGET_LIMIT) * 100
                    budget_status = '🟢' if budget_percentage < 70 else '🟡' if budget_percentage < 90 else '🔴'
                    
                    reset_percentage = (elapsed_minutes / total_session_minutes) * 100 if total_session_minutes > 0 else 0
                    
                    # Burn rates - use real rates calculated from actual conversation timelines
                    burn_rate = active_block.get('realBurnRateTokens', 0)
                    cost_burn_rate = active_block.get('realBurnRateCost', 0)
                    
                    # Time displays
                    reset_time = end_time.strftime('%H:%M')
                    
                    # Predict when tokens will run out
                    if burn_rate > 0 and total_tokens < TOKEN_LIMIT:
                        tokens_left = TOKEN_LIMIT - total_tokens
                        minutes_to_depletion = tokens_left / burn_rate
                        predicted_end = now + timedelta(minutes=minutes_to_depletion)
                        predicted_end_str = predicted_end.strftime('%H:%M')
                    elif total_tokens >= TOKEN_LIMIT:
                        predicted_end_str = "LIMIT HIT"
                    else:
                        predicted_end_str = reset_time
                    
                    # Status message
                    if total_tokens > TOKEN_LIMIT:
                        status_message = f"🚨 Session tokens exceeded limit! ({total_tokens:,} > {TOKEN_LIMIT:,})"
                    elif budget_percentage > 90:
                        status_message = "💸 High session cost!"
                    elif token_percentage > 90:
                        status_message = "🔥 High session usage!"
                    else:
                        status_message = "⛵ Smooth sailing..."
                    
                    # Add session count info if available
                    session_count = active_block.get('activeSessionCount', 1)
                    if session_count > 1:
                        status_message += f" ({session_count} active VMs)"
                    
                    # Display the monitor
                    print(f"⚡ Tokens:  {self.create_progress_bar(token_percentage, 20, token_status)} {total_tokens:,} / {TOKEN_LIMIT:,}")
                    print(f"💲 Budget:  {self.create_progress_bar(budget_percentage, 20, budget_status)} ${cost_used:.2f} / ${BUDGET_LIMIT:.2f}")
                    print(f"♻️  Reset:   {self.create_progress_bar(reset_percentage, 20, '🕐')} {self.format_time(remaining_minutes)}")
                    print()
                    # Display burn rates calculated from actual conversation timelines
                    burn_rate_str = f"{burn_rate:.1f} tok/min" if burn_rate > 0 else "0.0 tok/min"
                    cost_rate_str = f"${cost_burn_rate:.2f}/hour" if cost_burn_rate > 0 else "$0.00/hour"
                    print(f"🔥 {burn_rate_str} | 💰 {cost_rate_str}")
                    print()
                    print(f"🕐 {current_time} | 🏁 {predicted_end_str} | ♻️  {reset_time}")
                    print()
                    print(status_message)
                    
                    if snapshot:
                        print(f"\n[Snapshot mode - aggregated from {session_count} active session(s) across {len(self.discover_claude_paths())} Claude instances]")
                    
                except Exception as e:
                    print(f"❌ Error processing active session: {e}")
                    print("📝 Session data corrupted")
                    if snapshot:
                        print(f"\n[Snapshot mode - error occurred]")
            
            return
        
        # Snapshot mode - show data once and exit
        if snapshot:
            display_live_data()
            return
        
        # Live monitoring mode - continuous updates
        if not json_output:
            self.hide_cursor()
        
        # Graceful exit handling
        def signal_handler(signum, frame):
            if not json_output:
                self.show_cursor()
            print('\n\n\033[96mMonitoring stopped.\033[0m')
            sys.exit(0)
        
        signal.signal(signal.SIGINT, signal_handler)
        signal.signal(signal.SIGTERM, signal_handler)
        
        try:
            while True:
                display_live_data()
                
                # Wait 3 seconds before next update
                time.sleep(3)
                
        except KeyboardInterrupt:
            self.show_cursor()
            print('\n\n\033[96mMonitoring stopped.\033[0m')
        except Exception as e:
            self.show_cursor()
            print(f'\n\n❌ Monitor error: {e}')
        finally:
            self.show_cursor()

def parse_args():
    """Parse command line arguments"""
    parser = argparse.ArgumentParser(description='Claude Usage Max - Fast Python implementation')
    
    # Commands
    parser.add_argument('command', nargs='?', default='daily',
                      choices=['daily', 'monthly', 'session', 'blocks', 'live'],
                      help='Command to run (default: daily)')
    
    # Options
    parser.add_argument('--json', action='store_true', help='Output in JSON format')
    parser.add_argument('--last', type=int, help='Show last N entries')
    parser.add_argument('--snapshot', action='store_true', help='Show live data snapshot (single view, no monitoring loop)')
    parser.add_argument('--week', action='store_true', help='Show this week\'s data')
    parser.add_argument('--month', action='store_true', help='Show this month\'s data')
    parser.add_argument('--year', action='store_true', help='Show this year\'s data')
    parser.add_argument('--mode', choices=['auto', 'calculate', 'display'], 
                       default='auto', help='Cost calculation mode: auto (use costUSD if available, otherwise calculate), calculate (always calculate from tokens), display (always use costUSD)')
    
    # Date range filtering
    parser.add_argument('--since', type=str, help='Start date filter (YYYY-MM-DD)')
    parser.add_argument('--until', type=str, help='End date filter (YYYY-MM-DD)')
    
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
    if args.since:
        options['since'] = args.since
    if args.until:
        options['until'] = args.until
    if args.last:
        options['last'] = args.last
    
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
        # Handle live monitoring FIRST - don't scan historical data
        if args.command == 'live':
            if args.json and not args.snapshot:
                print("Error: Live monitoring does not support --json output", file=sys.stderr)
                sys.exit(1)
            analyzer.run_live_monitor(snapshot=args.snapshot, json_output=args.json)
            return
        
        # Add command to options for lazy processing
        options['command'] = args.command
        
        # Handle blocks command separately
        if args.command == 'blocks':
            # Load session blocks directly (without time filtering)
            blocks = analyzer.load_session_blocks(filter_recent=False)
            analyzer.display_blocks(blocks, args.last, args.json)
            return
        
        # Load data (for non-blocks, non-live commands)
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
            
    except Exception as e:
        if args.json:
            print(json.dumps({"error": str(e)}))
        else:
            print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == '__main__':
    main()