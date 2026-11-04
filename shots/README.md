Visual evidence for the "Make the full-screen lyrics cover text open the album or the artist" pull request.

Lives on this evidence branch only, so nothing lands in the pull request's
diff. Full-screen lyrics always draw dark (by design), so there is one theme;
the sizes cover the two layouts: wide (cover beside the lyrics) and narrow
(small cover in the heading).

- before/after shots (taken from the commit's parent and the commit) are
  byte-identical at both sizes: the change draws exactly the same view and
  only adds the hover underline and pointing cursor.
- after-click-title.png: clicking the title under the cover opened the album
  page and left full screen.
- after-click-artist.png: clicking the artist opened the artist page.
