@echo off
echo ========================================
echo   Push GitHub Pages (docs folder)
echo ========================================
echo.

REM Ajouter le dossier docs/ au staging
echo [1/3] Adding docs/ to git...
git add docs/

REM Commit avec un message
echo [2/3] Creating commit...
git commit -m "Update GitHub Pages - Phone Link interface"

REM Push vers GitHub
echo [3/3] Pushing to GitHub...
git push origin main

echo.
echo ========================================
echo   DONE! GitHub Pages will update in 1-2 minutes.
echo   URL: https://shazamifius.github.io/VRchatFACE-HAND-ARM-tracking/
echo ========================================
echo.

echo Don't forget to activate GitHub Pages in repository settings:
echo   - Go to: https://github.com/shazamifius/VRchatFACE-HAND-ARM-tracking/settings/pages
echo   - Source: Deploy from a branch
echo   - Branch: main / Folder: docs
echo   - Click Save
echo.

pause
