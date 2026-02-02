@echo off
echo [INFO] Lancement du test de precision OSC...
echo [INFO] Utilisation de Python 3.13 (Detecte via pip logs)...

REM Utilisation du chemin explicite trouve dans les logs pip
"C:\Users\shaza\AppData\Local\Programs\Python\Python313\python.exe" debug_osc_precision.py

if errorlevel 1 (
    echo.
    echo [ERREUR] Impossible de lancer le script avec Python 3.13.
    echo Essai avec 'py' launcher...
    py debug_osc_precision.py
)

pause
