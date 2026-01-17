# AllBeads Web Platform - Tech Stack Plan

**Date:** 2026-01-16
**Beads:** ab-6e7, ab-ibm
**Status:** Pending Approval

---

## Summary

Build the AllBeads web platform (allbeads.co) for team collaboration on beads. This plan analyzes tech stack options and recommends an approach balancing speed-to-market with long-term scalability.

---

## Tech Stack Options Analysis

### Option A: Rust Axum + Next.js Frontend
| Aspect | Details |
|--------|---------|
| **Code Sharing** | 60-70% of CLI code shareable via `allbeads-core` crate |
| **Existing Patterns** | Axum proven in `src/mail/server.rs` |
| **Real-time** | Manual WebSocket (tokio-tungstenite) |
| **Git Ops** | Native git2 integration |
| **Effort** | 10-12 weeks for MVP |
| **Best For** | Long-term, high-scale, code reuse |

### Option B: Python FastAPI + Next.js
| Aspect | Details |
|--------|---------|
| **Code Sharing** | None - must reimplement in Python |
| **Real-time** | WebSocket via FastAPI |
| **Git Ops** | GitPython or subprocess |
| **Effort** | 6-8 weeks for MVP |
| **Best For** | Rapid prototyping if team knows Python |

### Option C: Next.js Full-Stack (API Routes)
| Aspect | Details |
|--------|---------|
| **Code Sharing** | None - TypeScript only |
| **Existing Foundation** | website/ already has Next.js 14, Prisma, NextAuth |
| **Real-time** | Pusher/Ably or SSE |
| **Git Ops** | GitHub API (no direct git) |
| **Effort** | 4-6 weeks for MVP |
| **Best For** | Fastest launch, validate product-market fit |

### Option D: Convex + Next.js
| Aspect | Details |
|--------|---------|
| **Code Sharing** | None |
| **Real-time** | Built-in reactive subscriptions |
| **Git Ops** | Cannot clone repos (external worker needed) |
| **Effort** | 4-6 weeks for collaboration features |
| **Best For** | Real-time-first, simple CRUD apps |
| **Limitation** | Cannot perform git operations |

### Option E: Hybrid (Convex + Rust Worker)
| Aspect | Details |
|--------|---------|
| **Code Sharing** | Rust worker reuses CLI code |
| **Real-time** | Convex handles frontend sync |
| **Git Ops** | Rust worker does git clone/push |
| **Effort** | 8-10 weeks for MVP |
| **Best For** | Best of both worlds |

---

## Recommendation

### Short-Term (MVP): Option C - Next.js Full-Stack

**Why:**
1. Existing `website/` foundation (Next.js 14, Prisma, NextAuth, TanStack Query)
2. Fastest to market (4-6 weeks)
3. Validates product-market fit before complex investment
4. Use GitHub API for bead import (covers 80% of cases)

### Long-Term Migration Path

**Path 1: Rust Axum Backend (Option A)**
```
Month 1-2: MVP with Next.js
Month 3-4: Extract allbeads-core crate
Month 5-6: Build Axum API
Month 7+: Migrate frontend to Axum
```

**Path 2: Convex Hybrid (Option E)**
```
Month 1-2: MVP with Next.js
Month 3-4: Add Convex for real-time
Month 5-6: Build Rust git worker
Month 7+: Migrate to full Convex
```

---

## MVP Scope (Option C)

### Phase 1: Core Platform (Weeks 1-4)
- [ ] User auth (GitHub OAuth via NextAuth)
- [ ] Organization CRUD
- [ ] Project CRUD with repo linking
- [ ] Bead import via GitHub API
- [ ] Basic kanban board view
- [ ] Bead detail view

### Phase 2: Collaboration (Weeks 5-6)
- [ ] Web-only comments
- [ ] Basic real-time with SSE
- [ ] User profiles
- [ ] Activity feed

---

## Key Technical Decisions

### 1. Database Schema
Design Prisma schema to match Rust `Bead` struct for future migration:
```prisma
model Bead {
  id          String   @id
  title       String
  description String?
  status      String   // open, in_progress, blocked, closed
  priority    Int      @default(2)
  issueType   String
  createdAt   DateTime
  updatedAt   DateTime
  // ...
}
```

### 2. Git Sync Strategy
- MVP: GitHub API only (fetch issues.jsonl via Contents API)
- Future: Rust worker with git clone/push

### 3. Real-Time Strategy
- MVP: Server-Sent Events or Pusher
- Future: Convex subscriptions or WebSocket

### 4. API Design
Follow `specs/SPEC-allbeads-web.md` exactly for stable contracts.

---

## Files to Create/Modify

### Website Structure
```
website/
├── prisma/
│   └── schema.prisma          # Database models
├── src/
│   ├── app/
│   │   ├── page.tsx           # Landing page
│   │   ├── auth/
│   │   │   └── [...nextauth]/ # Auth routes
│   │   ├── dashboard/
│   │   │   └── page.tsx       # Main dashboard
│   │   ├── orgs/
│   │   │   └── [slug]/        # Org pages
│   │   ├── projects/
│   │   │   └── [id]/          # Project pages
│   │   └── api/
│   │       ├── orgs/          # Org API
│   │       ├── projects/      # Project API
│   │       └── beads/         # Bead API
│   ├── components/
│   │   ├── ui/                # Radix UI components
│   │   ├── kanban/            # Kanban board
│   │   └── bead/              # Bead cards/details
│   ├── lib/
│   │   ├── github.ts          # GitHub API client
│   │   ├── beads.ts           # Bead operations
│   │   └── db.ts              # Prisma client
│   └── types/
│       └── bead.ts            # TypeScript types
```

---

## Verification

1. Run `npm run dev` and verify landing page
2. Test GitHub OAuth login flow
3. Create org/project and verify in Prisma Studio
4. Import beads from a test repo via GitHub API
5. Verify kanban board displays beads correctly

---

## Detailed Trade-off Analysis

### Speed to Market

| Option | MVP Time | Why |
|--------|----------|-----|
| Next.js | 4-6 weeks | Existing foundation, single codebase |
| Convex + Next.js | 4-6 weeks | Real-time built-in, fast CRUD |
| FastAPI + Next.js | 6-8 weeks | Need to build backend from scratch |
| Hybrid (Convex + Rust) | 8-10 weeks | Two services to coordinate |
| Rust Axum + Next.js | 10-12 weeks | Extract shared crate, build API |

### Code Sharing with CLI

| Option | Sharing | Impact |
|--------|---------|--------|
| Rust Axum | 60-70% | Bead, ShadowBead, FederatedGraph, JSONL parsing, GitHub/JIRA adapters |
| Hybrid | 30-40% | Git worker shares graph/, storage/ modules |
| Next.js | 0% | Reimplement Bead types in TypeScript |
| FastAPI | 0% | Reimplement Bead types in Python |
| Convex | 0% | Reimplement Bead types in TypeScript |

### Real-Time Capabilities

| Option | Approach | Complexity |
|--------|----------|------------|
| Convex | Built-in reactive subscriptions | Low |
| Hybrid | Convex handles real-time layer | Low |
| Next.js | Add Pusher/Ably/SSE | Medium |
| Rust Axum | tokio-tungstenite WebSocket | High |
| FastAPI | FastAPI WebSocket | Medium |

### Git Operations

| Option | Approach | Limitation |
|--------|----------|------------|
| Rust Axum | Native git2 | None - full git ops |
| Hybrid | Rust worker with git2 | Async via worker |
| Next.js | GitHub API only | No direct git clone |
| FastAPI | GitPython | Slower, subprocess |
| Convex | Cannot do git | Needs external worker |

### Long-Term Maintainability

| Option | Languages | Services | Hiring |
|--------|-----------|----------|--------|
| Next.js | TypeScript | 1 | Easy |
| Convex | TypeScript | 1 | Easy |
| FastAPI | TypeScript + Python | 2 | Medium |
| Hybrid | TypeScript + Rust | 2 | Medium |
| Rust Axum | TypeScript + Rust | 2 | Harder |

### Vendor Lock-in

| Option | Lock-in | Migration Path |
|--------|---------|----------------|
| Next.js | Low (Vercel optional) | Standard Node.js |
| Rust Axum | None | Self-hosted anywhere |
| FastAPI | None | Standard Python |
| Convex | Medium | Export data, rewrite queries |
| Hybrid | Low-Medium | Rust worker is portable |

---

## Exploration Approach

Since you want to explore options, here's what I suggest:

### Option 1: Build a Quick Prototype in Each

Create minimal prototypes to feel the developer experience:

1. **Next.js API Route** - Create a simple bead CRUD endpoint
2. **Convex** - Set up Convex and create reactive bead queries
3. **Rust Axum** - Port the mail server pattern to serve beads

### Option 2: Start with Next.js, Design for Migration

1. Build MVP with Next.js (fastest)
2. Design interfaces that abstract the backend
3. When scale/needs clarify, migrate to preferred option

### Option 3: Spike on the Hardest Problem First

The hardest problem is **git sync with real-time updates**. Prototype this:
- How would Convex + Rust worker communicate?
- How would Next.js handle large repo clones?
- How would Axum handle concurrent git operations?

---

## My Suggestion

**Start with Next.js Full-Stack** (Option C) because:

1. You already have `website/` foundation
2. Fastest path to validate the product
3. GitHub API covers most bead import needs
4. Can always migrate later

Design the code with clean interfaces so migration is straightforward:

```typescript
// Abstract the bead source
interface BeadRepository {
  listBeads(projectId: string): Promise<Bead[]>
  getBead(id: string): Promise<Bead>
  syncFromGit(repoUrl: string): Promise<void>
}

// Start with GitHub API implementation
class GitHubBeadRepository implements BeadRepository { }

// Later: Switch to git clone implementation
class GitWorkerBeadRepository implements BeadRepository { }
```

This lets you ship fast and pivot when you know more.
