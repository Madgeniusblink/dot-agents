# Optional: add to a **project** repo .gitignore when you copy `.agents/`

```
.agents/var/scratch/**
!.agents/var/scratch/.gitignore
!.agents/var/scratch/README.md
.agents/var/worktrees/**
!.agents/var/worktrees/.gitignore
!.agents/var/worktrees/README.md
```

Global scratch still lives at `~/.agents/var/scratch/<repo-name>/` when you do not use project-local var.
