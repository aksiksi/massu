from django.http import HttpResponse, HttpResponseRedirect, HttpResponseNotAllowed
from django.shortcuts import render

from .forms import LaunchEmailForm


def index(request):
    form = LaunchEmailForm()
    return render(request, "web/index.html", {"form": form})


def pricing(request):
    return render(request, "web/pricing.html")


def faq(request):
    return render(request, "web/faq.html")


def mailing_list(request):
    if request.method == "POST":
        form = LaunchEmailForm(request.POST)
        if form.is_valid():
            form.save()
            return render(request, "web/launch_confirm.html")
    else:
        return HttpResponseNotAllowed(["POST"])
