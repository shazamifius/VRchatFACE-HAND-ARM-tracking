@echo off
echo.
echo ========================================================
echo   ATTENTION : CONFLIT DETECTE SUR GITHUB
echo ========================================================
echo.
echo Le depot distant (GitHub) contient des fichiers differents.
echo Comme vous m'avez demande de pousser VOTRE version locale,
echo ce script va FORCER l'envoi et ecraser la version distante.
echo.
echo Appuyez sur une touche pour confirmer l'ecrasement...
pause
git push -f origin main
echo.
echo Fini !
pause
