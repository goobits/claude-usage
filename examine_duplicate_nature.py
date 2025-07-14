#!/usr/bin/env python3
"""
Examine the nature of duplicate entries to understand what they represent
"""
import json
from pathlib import Path

def examine_specific_duplicates():
    """Look at actual duplicate entries to understand what they are"""
    
    # Focus on the file with the most duplicates
    jsonl_file = Path("/home/miko/.claude/projects/-home-miko-projects-utils-vm/c22b2d3d-3f10-4043-9e8e-ece7ef43db6a.jsonl")
    
    print("=" * 80)
    print("EXAMINING DUPLICATE ENTRIES IN DETAIL")
    print("=" * 80)
    
    # Look at the first duplicate group we found earlier
    target_lines = [5, 6, 7]  # msg_013ykVzuiBHZfJKFZYgNMi95:req_011CR2ADUzRLjvGeojrnvmbX
    
    print(f"🔍 Examining lines {target_lines} with same messageId+requestId")
    
    with open(jsonl_file, 'r') as f:
        lines = f.readlines()
        
        for line_num in target_lines:
            print(f"\n📄 LINE {line_num}:")
            line = lines[line_num - 1].strip()  # lines are 0-indexed
            
            try:
                data = json.loads(line)
                
                print(f"   Type: {data.get('type')}")
                print(f"   Message Role: {data.get('message', {}).get('role')}")
                print(f"   Message ID: {data.get('message', {}).get('id')}")
                print(f"   Request ID: {data.get('requestId')}")
                print(f"   UUID: {data.get('uuid')}")
                print(f"   Timestamp: {data.get('timestamp')}")
                print(f"   Parent UUID: {data.get('parentUuid')}")
                
                # Check if it's part of a conversation chain
                if 'message' in data and 'usage' in data['message']:
                    usage = data['message']['usage']
                    print(f"   Usage: {usage}")
                    
                    # Look at message content if it exists
                    if 'content' in data['message']:
                        content = data['message']['content']
                        if content:
                            if isinstance(content, list) and len(content) > 0:
                                first_content = content[0]
                                if isinstance(first_content, dict):
                                    content_type = first_content.get('type', 'unknown')
                                    print(f"   Content Type: {content_type}")
                                    
                                    if content_type == 'text':
                                        text = first_content.get('text', '')
                                        print(f"   Content Preview: {text[:100]}...")
                                    elif content_type == 'thinking':
                                        print(f"   Content: <thinking> block")
                            else:
                                print(f"   Content: {str(content)[:100]}...")
                
            except json.JSONDecodeError as e:
                print(f"   ERROR: Invalid JSON - {e}")
    
    print(f"\n" + "=" * 80)
    print("ANALYSIS QUESTIONS:")
    print("=" * 80)
    print("1. Do these represent the same API call logged multiple times?")
    print("2. Or different processing stages of the same request?")
    print("3. Are the UUIDs different because of internal processing?")
    print("4. Would Claude charge for each entry or just once?")
    
    # Look at different usage examples
    print(f"\n" + "=" * 80)
    print("EXAMINING DIFFERENT USAGE EXAMPLES:")
    print("=" * 80)
    
    # Look for a group with different usage data
    target_lines_different = [27, 28, 29]  # From fc2297b6 file - has different output tokens
    different_file = Path("/home/miko/.claude/projects/-home-miko-projects-utils-vm/fc2297b6-4ad7-4aa1-9586-d23afcf08f87.jsonl")
    
    if different_file.exists():
        print(f"🔍 Examining lines {target_lines_different} with DIFFERENT usage data")
        
        with open(different_file, 'r') as f:
            lines = f.readlines()
            
            for line_num in target_lines_different:
                if line_num <= len(lines):
                    print(f"\n📄 LINE {line_num}:")
                    line = lines[line_num - 1].strip()
                    
                    try:
                        data = json.loads(line)
                        
                        print(f"   Message ID: {data.get('message', {}).get('id')}")
                        print(f"   Request ID: {data.get('requestId')}")
                        print(f"   UUID: {data.get('uuid')}")
                        print(f"   Timestamp: {data.get('timestamp')}")
                        
                        if 'message' in data and 'usage' in data['message']:
                            usage = data['message']['usage']
                            print(f"   Usage: {usage}")
                            
                            # Calculate cost for this specific entry
                            cost = (usage.get('input_tokens', 0) * 3e-06 + 
                                   usage.get('output_tokens', 0) * 1.5e-05 +
                                   usage.get('cache_creation_input_tokens', 0) * 3.75e-06 +
                                   usage.get('cache_read_input_tokens', 0) * 3e-07)
                            print(f"   Estimated cost: ${cost:.6f}")
                            
                    except json.JSONDecodeError as e:
                        print(f"   ERROR: Invalid JSON - {e}")
    
    print(f"\n" + "=" * 80)
    print("KEY QUESTIONS FOR BILLING:")
    print("=" * 80)
    print("❓ If these entries have:")
    print("  - Same messageId + requestId = Should they be billed once?")
    print("  - Different UUIDs = Are they separate billable events?")
    print("  - Different timestamps = Are they separate API calls?")
    print("  - Different usage data = Are they definitely separate charges?")

if __name__ == "__main__":
    examine_specific_duplicates()