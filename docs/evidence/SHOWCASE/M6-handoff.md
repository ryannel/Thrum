# SHOWCASE M6 handoff

The complete builder checkpoint is `390df4601` on `build/showcase`. It records
all six builder results as pass and leaves M6 at `in_progress`.
`docs/evidence/SHOWCASE/M6.md` on that branch links the Metal and Chrome
equality records, four 1080p paired timing runs, and the elapsed engine
captures and lossless inspection windows. Both Metal recipes matched all
1,351 submitted frames, and Chrome matched all six checkpoints. The timing
pairs and resolved selections support Frustum on both targets. The
registered exact-image verifier uses the shared main-branch check.

The workspace, Web and documentation checks passed in the builder
worktree. Both equality commands passed under their resource locks. The
movies and inspection archive restored with the registered hashes from
the local evidence store. The motion takes came from
`77edc576ee688921008068605e25bd7d4bfadda0`, with the same capture inputs as
the full checkpoint. The capture input digest was
`00fe0227e0420af5bd40417e3b49eeae1f62950a05b9b1039a48486ac9e6b5fd`.

The checked merge of the full checkpoint entered the queue at 01:58 local
time. As the two-hour builder turn approached its limit, the batch ahead
was still testing clean main at `de59644d6`, in
`mixed_mesh_occlusion_matches_bucket_direct_through_first_cut`.
SHOWCASE's merge had returned no result. I stopped my queued request
before committing this handoff. The runner had reported no SHOWCASE
failure and had made no clean-main failure ruling.

Submit the current branch with
`python3 "$ORCHESTRATE_SCRIPT" merge --worktree .` when the queue can run it.
Main advanced through HOST and RENDER-2D, touching shared renderer and
capture paths. Retake the Mac flythrough, Mac fast yaw and Chrome
flythrough on the merged runtime, inspect them and merge the refreshed
evidence. The evidence page has the reproduction commands. The current
worktree also has the capture and packaging scripts in `tmp/showcase-m6/`.
End with `DONE` once those steps land, so the loop runs the checks on
main and assigns the milestone status.
