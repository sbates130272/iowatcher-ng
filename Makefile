.PHONY: deploy
deploy:
	@rm -rf /tmp/book
	@git worktree prune
	@echo "====> deploying to github"
	git worktree add /tmp/book gh-pages
	mdbook build
	rm -rf /tmp/book/*
	cp -rp docs/* /tmp/book/
	cd /tmp/book && \
		git update-ref -d refs/heads/gh-pages ; \
		git add -fA && \
		git commit -m "Update build."