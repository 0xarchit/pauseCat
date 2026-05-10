import os
import subprocess
import json
import requests
import time
import sys

MODEL_URL = "https://models.github.io/inference/chat/completions"
MODEL_NAME = "gpt-4o-mini"
MAX_CHARS_PER_CHUNK = 15000
MAX_DIFF_CHARS = 1500
INCLUDED_PATHS = ["src/*", "wix/*", "build.rs", "Cargo.toml"]

def get_commit_data():
    try:
        tags = subprocess.check_output(["git", "tag", "--sort=-creatordate"]).decode().split()
        if not tags:
            log_range = ["git", "log", "--pretty=format:%h"]
        elif len(tags) >= 2:
            log_range = ["git", "log", f"{tags[1]}..{tags[0]}", "--pretty=format:%h"]
        else:
            log_range = ["git", "log", tags[0], "--pretty=format:%h"]
            
        hashes = subprocess.check_output(log_range).decode(errors='ignore').split()
        commit_data = []
        
        for h in hashes[:100]:
            msg = subprocess.check_output(["git", "show", "-s", "--format=%s", h]).decode(errors='ignore').strip()
            
            show_cmd = ["git", "show", "--patch", "--stat", "--format=", h, "--"] + INCLUDED_PATHS
            diff = subprocess.check_output(show_cmd).decode(errors='ignore')
            
            if not diff.strip():
                continue
                
            diff = diff[:MAX_DIFF_CHARS] + "\n...[truncated]" if len(diff) > MAX_DIFF_CHARS else diff
            commit_data.append(f"Commit: {h}\nMessage: {msg}\nChanges:\n{diff}")
            
        return commit_data
    except Exception as e:
        print(f"Error: {e}")
        return []

def call_ai_with_retries(payload, api_key, max_retries=3):
    headers = {
        "Authorization": f"Bearer {api_key}",
        "Content-Type": "application/json"
    }
    for attempt in range(max_retries):
        try:
            response = requests.post(MODEL_URL, headers=headers, json=payload, timeout=45)
            if response.status_code == 429:
                time.sleep((attempt + 1) * 20)
                continue
            response.raise_for_status()
            return response.json()['choices'][0]['message']['content']
        except Exception as e:
            if attempt < max_retries - 1:
                time.sleep(5 * (attempt + 1))
            else:
                raise e
    return None

def main():
    output_file = "release_notes.md"
    api_key = os.getenv("GH_MODELS_API_KEY")
    commit_data = get_commit_data()
    
    raw_lines = []
    for c in commit_data:
        lines = c.split('\n')
        if len(lines) > 1:
            raw_lines.append(lines[1])
    raw_changelog = "## Commits\n" + "\n".join(raw_lines) if raw_lines else "Maintenance release."

    if not api_key:
        with open(output_file, "w", encoding="utf-8") as f:
            f.write(raw_changelog)
        return

    if not commit_data:
        with open(output_file, "w", encoding="utf-8") as f:
            f.write("System optimizations and stability improvements.")
        return

    try:
        chunks = []
        current_chunk = ""
        for data in commit_data:
            if len(current_chunk) + len(data) > MAX_CHARS_PER_CHUNK:
                chunks.append(current_chunk)
                current_chunk = data + "\n\n"
            else:
                current_chunk += data + "\n\n"
        if current_chunk:
            chunks.append(current_chunk)

        summaries = []
        for chunk in chunks:
            payload = {
                "model": MODEL_NAME,
                "messages": [
                    {"role": "system", "content": "You are a technical lead. Summarize these commits and their code changes into clear bullet points. Focus on Features, Fixes, and Refactors. No emojis. Technical tone only."},
                    {"role": "user", "content": f"Commits and diffs:\n{chunk}"}
                ],
                "temperature": 0.2
            }
            summary = call_ai_with_retries(payload, api_key)
            if summary:
                summaries.append(summary)
            time.sleep(2)

        combined_summaries = "\n\n".join(summaries)
        final_payload = {
            "model": MODEL_NAME,
            "messages": [
                {
                    "role": "system", 
                    "content": "You are a professional software release manager. Create a high-quality GitHub release description from the provided summaries. Use headers: ## Key Features, ## Bug Fixes, and ## Technical Improvements. Strictly NO emojis. Use a clean, engineering-focused tone."
                },
                {"role": "user", "content": f"Partial summaries:\n{combined_summaries}"}
            ],
            "temperature": 0.4
        }
        final_notes = call_ai_with_retries(final_payload, api_key)
        if final_notes:
            with open(output_file, "w", encoding="utf-8") as f:
                f.write(final_notes)
        else:
            raise Exception("Empty AI response")
    except Exception as e:
        print(f"Fallback: {e}")
        with open(output_file, "w", encoding="utf-8") as f:
            f.write(raw_changelog)

if __name__ == "__main__":
    main()
