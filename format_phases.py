import sys

def format_phases():
    with open('docs/phases.md', 'r', encoding='utf-8') as f:
        lines = f.read().splitlines()

    out = []
    in_milestone = False
    
    categories = {
        "Objectives", "Deliverables", "Execution Engine", "Bytecode Loader", "Core Opcodes", "Debugging",
        "Runtime Values", "Heap System", "Exception System", "Native Call System",
        "Filesystem", "Networking", "Process", "Environment", "Timing", "Dynamic Libraries", "Supported Platforms",
        "Collections", "Utilities", "Serialization", "File APIs", "Networking APIs",
        "Scheduler", "Event Loop", "Async APIs",
        "Permissions", "Sandbox", "Examples",
        "Module Loader", "Package Manager", "Features:", "Potential future:",
        "Build VIR", "VIR Features", "Optimization Passes", "Optimizations",
        "Tier 1", "Tier 2", "Tier 3", "Targets",
        "Compiler", "APIs", "Goal",
        "Features", "Challenges",
        "UI Components", "Rendering", "Platforms",
        "Android", "iOS",
        "Server Features", "Deployments",
        "Developer Tools", "Community", "Governance"
    }
    
    for i, line in enumerate(lines):
        stripped = line.strip()
        if not stripped:
            out.append(line)
            continue
            
        if stripped == 'Milestone Summary':
            out.append(f"## {stripped}")
            in_milestone = True
            continue
            
        if in_milestone:
            parts = stripped.split('  ', 1)
            if len(parts) == 2 and parts[0].isdigit():
                out.append(f"- **{parts[0]}** {parts[1]}")
            else:
                out.append(f"- {stripped}")
            continue

        if stripped.startswith('Phase '):
            out.append(f"## {stripped}")
            continue
            
        if stripped.startswith('Status:'):
            out.append(f"**{stripped}**")
            continue
            
        if stripped in ['?']:
            out.append(f"    {stripped}")
            continue
            
        if stripped in categories or stripped.endswith(':'):
            out.append(f"### {stripped}")
        elif stripped in ['Vyauma', 'VIR', 'VRE Bytecode', 'VRE', 'TypeScript', 'Bytecode']:
            is_flow = False
            if i + 1 < len(lines) and lines[i+1].strip() == '?':
                is_flow = True
            if i - 1 >= 0 and lines[i-1].strip() == '?':
                is_flow = True
            if is_flow:
                out.append(f"    {stripped}")
            else:
                out.append(f"- {stripped}")
        elif stripped.startswith('vre run'):
            out.append(f"    {stripped}")
        elif stripped in [
            'This is where VRE becomes language-agnostic.',
            'First external language.',
            'This is where VRE becomes a real Electron alternative.',
            'Only after VRE is mature.',
            'Most important next phase.'
        ]:
            out.append(f"*{stripped}*")
        else:
            out.append(f"- {stripped}")

    with open('docs/phases.md', 'w', encoding='utf-8') as f:
        f.write('\n'.join(out) + '\n')

format_phases()
