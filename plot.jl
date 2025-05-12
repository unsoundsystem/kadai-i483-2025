using Plots, CSV, StatsPlots, DataFrames
if length(ARGS) == 0
    println("data file not specified")
    exit(1)
end
data=CSV.read(ARGS[1], DataFrame)

x = 0:size(data)[1]-1
lx_plt = plot(x, [data.BH1750_lx data.RPR0521RS_lx data.RPR0521RS_inf], label=["BH1750 lx" "RPR0521RS lx" "RPR0521RS infrated"],  legend=:topleft)
humidity_plt = plot(x, data.SCD41_humidity, label="SCD41 humidity (RH)")
co2_plt = plot(x, data.SCD41_co2, label="SCD41 CO2 (ppm)")
tmp_plot = plot(x, [data.DPS310_temp data.SCD41_temp], label=["DPS310 temp" "SCD41 temp"])
pres_plt = plot(data.DPS310_pres)

savefig(pres_plt, "~/projects/smart-embedded-system/kadai-i483-2025/img/dps_pres.png")
savefig(lx_plt, "~/projects/smart-embedded-system/kadai-i483-2025/img/lx.png")
savefig(tmp_plot, "~/projects/smart-embedded-system/kadai-i483-2025/img/temp.png")
savefig(co2_plt, "~/projects/smart-embedded-system/kadai-i483-2025/img/co2.png")
savefig(humidity_plt, "~/projects/smart-embedded-system/kadai-i483-2025/img/humidity.png")
