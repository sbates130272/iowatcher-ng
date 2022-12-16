.PHONY: deploy
deploy:
	@git worktree prune
	@echo "====> deploying to github"
	git worktree add /tmp/book gh-pages
	cd /tmp/book; mdbook build; rm -rf /tmp/book/*; cp -rp book/* /tmp/book/
	cd /tmp/book && \
		git update-ref -d refs/heads/gh-pages ; \
		git add -fA && \
		git commit -m "deployed on $(shell date) by ${USER}" && \
		git push --force origin gh-pages