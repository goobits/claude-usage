# Claude Usage Limit Detection Analysis Report

## Executive Summary

This report analyzes Claude JSONL conversation files to identify patterns that indicate usage limits have been hit. These patterns can be used to implement automatic detection of "plan-worthy" conversations for sliding window implementation.

## Data Sources Analyzed

- **Files Analyzed**: 2 JSONL conversation files
- **Total Entries**: 2,431 entries (assistant: 1,475, user: 942, system: 2, summary: 12)
- **Conversations with Limit Patterns**: 2 conversations
- **Total Pattern Instances Found**: 449 instances

## Key Findings

### 1. Explicit System Limit Messages

**Pattern**: System warning messages indicating model limits have been reached.

```json
{
  "type": "system",
  "level": "warning", 
  "content": "Claude Opus 4 limit reached, now using Sonnet 4",
  "timestamp": "2025-07-15T21:08:57.627Z"
}
```

**Characteristics**:
- Type: `system`
- Level: `warning`
- Content contains: "limit reached", "now using"
- Exact pattern: "Claude Opus 4 limit reached, now using Sonnet 4"
- **Instances Found**: 2

### 2. User Interruptions

**Pattern**: User manually interrupting or stopping Claude's response.

```json
{
  "type": "user",
  "message": {
    "content": [{"type": "text", "text": "[Request interrupted by user]"}]
  },
  "timestamp": "2025-07-15T20:44:09.321Z"
}
```

**Characteristics**:
- Type: `user`
- Content contains: "[Request interrupted by user]", "interrupted"
- **Instances Found**: 1

### 3. High Token Usage Responses

**Pattern**: Assistant responses with extremely high token counts indicating approaching limits.

**Characteristics**:
- Type: `assistant`
- Total tokens > 100,000 (input + output + cache tokens)
- Often clustered in sequences
- **Instances Found**: 378 high token responses
- **Typical Range**: 100,000 - 150,000 tokens per response

**Token Distribution**:
- Line 329: 100,111 tokens (claude-opus-4-20250514)
- Line 330: 100,111 tokens (claude-opus-4-20250514)
- Line 332: 100,607 tokens (claude-opus-4-20250514)

### 4. Model Downgrades

**Pattern**: Automatic switching from higher-tier to lower-tier models due to limits.

**Characteristics**:
- Model change from `claude-opus-4-20250514` → `claude-sonnet-4-20250514`
- Often occurs after high token usage
- **Instances Found**: 14 model downgrades

**Examples**:
- Lines 395-398: Opus → Sonnet (114,783 → 115,591 tokens)
- Lines 835-837: Opus → Sonnet (12,120 → 21,252 tokens)

### 5. Sudden Token Drops

**Pattern**: Dramatic decreases in token usage between consecutive responses.

**Characteristics**:
- Previous response > 50,000 tokens
- Current response < 50% of previous
- Drop percentages: 51.4% - 99.9%
- **Instances Found**: 13 sudden drops

**Examples**:
- Lines 458-464: 133,957 → 25,460 (81.0% drop)
- Lines 826-831: 130,182 → 29,252 (77.5% drop)
- Lines 254-260: 127,407 → 33,685 (73.6% drop)

### 6. Time Clustering

**Pattern**: High concentration of responses with significant token usage within short time windows.

**Characteristics**:
- 3+ responses within 5-minute windows
- Total tokens in window > 200,000
- **Instances Found**: 41 time clusters

**Examples**:
- 2025-07-15 20:55:00: 70 responses, 5,547,476 total tokens (79,250 avg)
- 2025-07-15 20:50:00: 39 responses, 2,204,592 total tokens (56,528 avg)

## JSON Structure Analysis

### Assistant Response with Usage Data
```json
{
  "type": "assistant",
  "message": {
    "id": "msg_018JhBAKijfvLB8GBAsyTYCQ",
    "model": "claude-opus-4-20250514",
    "usage": {
      "input_tokens": 10,
      "cache_creation_input_tokens": 5051,
      "cache_read_input_tokens": 10369,
      "output_tokens": 4,
      "service_tier": "standard"
    },
    "stop_reason": "end_turn"
  },
  "sessionId": "b2355a52-4b6d-47f1-a5a9-f6d786aaae6a",
  "timestamp": "2025-07-15T07:59:54.740Z"
}
```

### System Limit Warning
```json
{
  "type": "system",
  "content": "Claude Opus 4 limit reached, now using Sonnet 4",
  "level": "warning",
  "sessionId": "f6bc4859-f162-4d72-9159-a39ef5d9594a",
  "timestamp": "2025-07-15T21:08:57.627Z"
}
```

## Sliding Window Detection Algorithm

Based on the analysis, implement these detection patterns for identifying plan-worthy conversations:

### 1. Explicit Limit Detection (Highest Priority)
```python
def detect_explicit_limits(entry):
    return (entry.get('type') == 'system' and 
            entry.get('level') == 'warning' and
            'limit reached' in entry.get('content', '').lower())
```

### 2. Token Threshold Detection
```python
def detect_high_token_usage(entry):
    if entry.get('type') != 'assistant':
        return False
    usage = entry.get('message', {}).get('usage', {})
    total_tokens = (usage.get('input_tokens', 0) + 
                   usage.get('output_tokens', 0) + 
                   usage.get('cache_read_input_tokens', 0) +
                   usage.get('cache_creation_input_tokens', 0))
    return total_tokens > 100000
```

### 3. Model Downgrade Detection
```python
def detect_model_downgrade(prev_entry, curr_entry):
    if (prev_entry.get('type') == 'assistant' and 
        curr_entry.get('type') == 'assistant'):
        prev_model = prev_entry.get('message', {}).get('model', '')
        curr_model = curr_entry.get('message', {}).get('model', '')
        return ('opus' in prev_model.lower() and 
                'sonnet' in curr_model.lower())
    return False
```

### 4. Sudden Drop Detection
```python
def detect_sudden_drop(prev_entry, curr_entry):
    def get_total_tokens(entry):
        usage = entry.get('message', {}).get('usage', {})
        return (usage.get('input_tokens', 0) + 
                usage.get('output_tokens', 0) + 
                usage.get('cache_read_input_tokens', 0) +
                usage.get('cache_creation_input_tokens', 0))
    
    if (prev_entry.get('type') == 'assistant' and 
        curr_entry.get('type') == 'assistant'):
        prev_tokens = get_total_tokens(prev_entry)
        curr_tokens = get_total_tokens(curr_entry)
        
        return (prev_tokens > 50000 and 
                curr_tokens < prev_tokens * 0.5)
    return False
```

### 5. User Interruption Detection
```python
def detect_user_interruption(entry):
    if entry.get('type') != 'user':
        return False
    content = entry.get('message', {}).get('content', '')
    if isinstance(content, list):
        content = ' '.join([item.get('text', '') for item in content])
    
    return any(pattern in content.lower() for pattern in [
        '[request interrupted by user]',
        'interrupted',
        'stop generating',
        'please stop'
    ])
```

## Implementation Recommendations

### Priority Levels for Plan Detection

1. **IMMEDIATE (Auto-trigger)**: 
   - System limit messages
   - User interruptions

2. **HIGH (Strong indicator)**:
   - Token usage > 100k with model downgrades
   - Multiple consecutive high-token responses

3. **MEDIUM (Pattern-based)**:
   - Sudden token drops > 70%
   - Time clustering with high token density

4. **LOW (Contextual)**:
   - Isolated high token responses
   - Minor model changes

### Sliding Window Parameters

- **Window Size**: 10-20 conversation turns
- **Token Threshold**: 100,000 tokens per response
- **Drop Threshold**: 50% decrease in token usage
- **Time Window**: 5-minute clustering analysis
- **Model Tier Tracking**: Monitor Opus → Sonnet transitions

## Message IDs and Special Fields for Tracking

### Key Fields for Limit Detection:
- `type`: Message type (system, assistant, user)
- `level`: System message level (warning for limits)
- `content`: System message content
- `message.usage.input_tokens`: Input token count
- `message.usage.output_tokens`: Output token count
- `message.usage.cache_read_input_tokens`: Cache token count
- `message.model`: Model identifier
- `message.stop_reason`: Response termination reason
- `timestamp`: Message timestamp for temporal analysis

### Session Tracking:
- `sessionId`: Unique session identifier
- `uuid`: Unique message identifier
- `parentUuid`: Parent message for threading

## Conclusion

The analysis reveals clear, detectable patterns when Claude hits usage limits. The most reliable indicators are:

1. **System warning messages** with "limit reached" content
2. **High token usage** (>100k tokens) in assistant responses
3. **Model downgrades** from Opus to Sonnet
4. **Sudden token drops** after high usage periods
5. **User interruptions** of responses

These patterns can be implemented in a sliding window algorithm to automatically detect conversations that would benefit from plan mode, improving user experience by proactively suggesting workflow optimization.

The detection system should prioritize explicit system messages and user interruptions as immediate triggers, while using token patterns and model changes as supporting evidence for plan-worthy conversation identification.