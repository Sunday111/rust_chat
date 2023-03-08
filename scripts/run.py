from pathlib import Path
import subprocess

SCRIPT_DIR = Path(__file__).parent.resolve()
PROJECT_DIR = SCRIPT_DIR.parent

def main():
    subprocess.run(check=True,cwd=PROJECT_DIR, args=[
        'cargo', 'build'
    ])

    subprocess.Popen(['cargo', 'run', '-p', 'rust_chat_server'], cwd=PROJECT_DIR)

    client_count = 2
    for _ in range(client_count):
        subprocess.Popen(['cargo', 'run', '-p', 'rust_chat_client'], cwd=PROJECT_DIR)

if __name__ == '__main__':
    main()
