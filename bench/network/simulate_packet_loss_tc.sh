#!/usr/bin/env bash
# Simulación de cortes de red y degradación de enlace usando tc-netem (ISO/IEC 25010 - Fiabilidad)
# Requiere permisos de root o privilegios de red en contenedor

IFACE=${1:-"eth0"}
ACTION=${2:-"loss"}

case $ACTION in
  "loss")
    echo "Aplicando 20% de pérdida de paquetes en interfaz $IFACE..."
    sudo tc qdisc add dev $IFACE root netem loss 20%
    ;;
  "delay")
    echo "Aplicando latencia de 150ms (+/- 30ms jitter) en interfaz $IFACE..."
    sudo tc qdisc add dev $IFACE root netem delay 150ms 30ms
    ;;
  "reset")
    echo "Restableciendo condiciones de red normales en interfaz $IFACE..."
    sudo tc qdisc del dev $IFACE root
    ;;
  *)
    echo "Uso: $0 <interfaz> [loss|delay|reset]"
    exit 1
    ;;
esac
