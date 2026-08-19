import sys

def main():
    code_lines = []
    
    # Read until the sentinel token is encountered
    for line in sys.stdin:
        if line.strip() == "===END_OF_WORKER_CODE===":
            break
        code_lines.append(line)
        
    worker_code = "".join(code_lines)
    # Execute the code in a private dictionary namespace
    worker_scope = {}
    try:
        exec(worker_code, worker_scope)
        process_fn = worker_scope.get("get_chess_move")
        if not callable(process_fn):
            print("ERROR: Function 'process' not found in provided code", flush=True)
            return
    except Exception as e:
        print(f"ERROR: Failed to load worker: {e}", flush=True)
        return
    # Signal to the host that the worker is loaded and ready
    print("READY", flush=True)

    while True:
        line = sys.stdin.readline()
        
        if not line or len(line) < 2:
            continue

        payload = line.strip()
        print("received: " + payload)
        if payload == "kill":
            break

        try:
            result = process_fn(payload)
            print("move: "+result, flush=True)
        except Exception as e:
            print(f"ERROR: {e}", flush=True)

if __name__ == "__main__":
    main()